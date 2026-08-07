"""wa-invite — send rinvite e-invites over WhatsApp from a locally paired account."""

from __future__ import annotations

import argparse
import os
import random
import sys
import time
from dataclasses import dataclass
from typing import Any

from wa_invite import __version__, links, phone, template
from wa_invite.api import RinviteApi
from wa_invite.config import Config, load
from wa_invite.console import console, error, info, panel, table, warn
from wa_invite.errors import ConfigError, WaInviteError, WhatsAppError
from wa_invite.ledger import FAILED, SENT, SKIPPED, Ledger
from wa_invite.template import ordered_names


@dataclass
class Recipient:
    guest: dict[str, Any]
    invite_url: str | None = None
    body: str | None = None
    digits: str | None = None
    skip_reason: str | None = None

    @property
    def id(self) -> str:
        return self.guest["id"]

    @property
    def name(self) -> str:
        return self.guest["name"]

    @property
    def sendable(self) -> bool:
        return self.skip_reason is None and bool(self.body) and bool(self.digits)


# --- shared preparation --------------------------------------------------


def couple_of(event: dict[str, Any]) -> str:
    first, second = ordered_names(event)
    return f"{first} & {second}"


def prepare(
    config: Config, api: RinviteApi, event_id: str, *, only: str | None = None
) -> tuple[dict[str, Any], list[Recipient]]:
    """Fetch the event and its einvite guests, resolving links and messages.

    Anything that cannot be turned into a message gets a `skip_reason` rather
    than being dropped, so no guest silently disappears from the report.
    """
    event = api.get_event(event_id)
    guests = api.list_einvite_guests(event_id)

    if only:
        needle = only.lower()
        guests = [
            g
            for g in guests
            if needle in g["name"].lower() or needle in (g.get("phone") or "")
        ]

    body_template = template.load_template(config.template_file)
    recipients: list[Recipient] = []
    insecure_warned = False

    for guest in guests:
        recipient = Recipient(guest=guest)
        try:
            recipient.invite_url = links.resolve(
                guest["invite_url"], config.invite_base_url
            )
        except ValueError as exc:
            raise ConfigError(str(exc)) from exc

        if not insecure_warned and links.is_insecure(recipient.invite_url):
            warn(
                f"invite links are not https ({recipient.invite_url}) — "
                f"guests may not be able to open them"
            )
            insecure_warned = True

        recipient.body = template.render(
            body_template, event, guest, recipient.invite_url
        )

        try:
            recipient.digits = phone.normalize(
                guest.get("phone"), config.default_country_code
            )
        except phone.PhoneError as exc:
            recipient.skip_reason = str(exc)

        recipients.append(recipient)

    return event, recipients


# --- commands ------------------------------------------------------------


def cmd_login(args: argparse.Namespace) -> int:
    from wa_invite.whatsapp import WhatsAppSession, render_qr

    config = load(require_api=False)
    info(f"Pairing a WhatsApp device. Session: [cyan]{config.session_db}[/cyan]")
    info("On your phone: WhatsApp → Settings → Linked devices → Link a device\n")

    session = WhatsAppSession(config.session_db)

    def show_qr(data: str) -> None:
        console.print(render_qr(data))
        info("[dim]Scan the code above. It refreshes every ~20s.[/dim]")

    try:
        number = session.pair(show_qr)
    finally:
        session.close()

    info(f"\n[green]Paired.[/green] Messages will be sent from [bold]{number}[/bold].")
    return 0


def cmd_events(args: argparse.Namespace) -> int:
    config = load()
    with RinviteApi(config) as api:
        events = api.list_events()
    table(
        "Your events",
        ["ID", "Couple", "Date", "Hall", "Venue"],
        [
            [e["id"], couple_of(e), e["event_date"], e["hall_name"], e["venue_name"]]
            for e in events
        ],
    )
    return 0


def cmd_guests(args: argparse.Namespace) -> int:
    config = load()
    with RinviteApi(config) as api:
        event = api.get_event(args.event)
        guests = api.list_guests(args.event)
    if not args.all:
        guests = [g for g in guests if g["channel"] == "einvite"]

    with Ledger(config.ledger_db) as ledger:
        already_sent = ledger.sent_guest_ids(args.event)

    rows = []
    for guest in guests:
        try:
            number = phone.display(
                phone.normalize(guest.get("phone"), config.default_country_code)
            )
        except phone.PhoneError as exc:
            number = f"[red]{exc}[/red]"
        rows.append(
            [
                guest["name"],
                guest["channel"],
                number,
                guest["rsvp_status"],
                SENT if guest["id"] in already_sent else "pending",
                links.resolve(guest["invite_url"], config.invite_base_url),
            ]
        )

    scope = "all guests" if args.all else "e-invite guests"
    table(
        f"{couple_of(event)} — {scope} ({len(rows)})",
        ["Name", "Channel", "Phone", "RSVP", "WhatsApp", "Invite link"],
        rows,
    )
    return 0


def cmd_preview(args: argparse.Namespace) -> int:
    config = load()
    with RinviteApi(config) as api:
        event, recipients = prepare(config, api, args.event, only=args.only)

    if not recipients:
        warn("no e-invite guests matched")
        return 0

    for recipient in recipients[: args.limit]:
        target = (
            phone.display(recipient.digits)
            if recipient.digits
            else f"[red]{recipient.skip_reason}[/red]"
        )
        panel(recipient.body or "", f"{recipient.name} → {target}")

    remaining = len(recipients) - min(args.limit, len(recipients))
    if remaining > 0:
        info(f"[dim]… and {remaining} more. Use --limit to see them.[/dim]")
    return 0


def cmd_status(args: argparse.Namespace) -> int:
    config = load(require_api=False)
    with Ledger(config.ledger_db) as ledger:
        entries = ledger.entries(args.event)
    table(
        f"Send ledger for event {args.event}",
        ["Guest", "Phone", "Status", "When", "Detail"],
        [
            [e.guest_name, e.phone, e.status, e.sent_at, e.detail or ""]
            for e in entries
        ],
    )
    counts: dict[str, int] = {}
    for entry in entries:
        counts[entry.status] = counts.get(entry.status, 0) + 1
    if counts:
        info(" ".join(f"{status}={count}" for status, count in sorted(counts.items())))
    return 0


def cmd_send(args: argparse.Namespace) -> int:
    from wa_invite.whatsapp import WhatsAppSession

    config = load()

    with RinviteApi(config) as api:
        event, recipients = prepare(config, api, args.event, only=args.only)

    if not recipients:
        warn("no e-invite guests matched")
        return 0

    ledger = Ledger(config.ledger_db)
    try:
        already_sent = ledger.sent_guest_ids(args.event)
        if not args.resend:
            for recipient in recipients:
                if recipient.id in already_sent and recipient.skip_reason is None:
                    recipient.skip_reason = "already sent (use --resend to override)"

        candidates = [r for r in recipients if r.sendable]
        skipped = [r for r in recipients if not r.sendable]

        if args.limit is not None:
            held_back = candidates[args.limit :]
            candidates = candidates[: args.limit]
            for recipient in held_back:
                recipient.skip_reason = f"beyond --limit {args.limit}"
                skipped.append(recipient)

        if not candidates:
            report(event, [], skipped)
            return 0

        session: WhatsAppSession | None = None
        sent: list[Recipient] = []
        failed: list[Recipient] = []
        try:
            sender = "dry run"
            if not args.dry_run:
                session = WhatsAppSession(config.session_db)
                session.connect()
                sender = session.own_number() or "unknown number"

                # Catches landlines, typos and a wrong country code before we
                # send anything to them.
                registered = session.registered_numbers([r.digits for r in candidates])
                reachable: list[Recipient] = []
                for recipient in candidates:
                    if registered.get(recipient.digits):
                        reachable.append(recipient)
                    else:
                        recipient.skip_reason = "no WhatsApp account for this number"
                        skipped.append(recipient)
                candidates = reachable

            if candidates and confirm(event, candidates, skipped, sender, args):
                sent, failed = deliver(config, ledger, args, candidates, session)
            elif candidates and not args.dry_run:
                info("Aborted. Nothing was sent.")
                return 0
            else:
                # Dry run, or nothing reachable: everything still pending is a skip.
                for recipient in candidates:
                    if recipient.skip_reason is None:
                        recipient.skip_reason = (
                            "dry run" if args.dry_run else "not reachable"
                        )
                skipped = candidates + skipped
        finally:
            if session is not None:
                session.close()

        report(event, sent, skipped, failed)
        return 1 if failed else 0
    finally:
        ledger.close()


def confirm(
    event: dict[str, Any],
    candidates: list[Recipient],
    skipped: list[Recipient],
    sender: str,
    args: argparse.Namespace,
) -> bool:
    panel(candidates[0].body or "", f"Example message → {candidates[0].name}")
    info(
        f"\nAbout to send [bold]{len(candidates)}[/bold] WhatsApp message(s) "
        f"from [bold]{sender}[/bold] to guests of [bold]{couple_of(event)}[/bold]."
    )
    if skipped:
        info(f"[yellow]{len(skipped)} guest(s) will be skipped.[/yellow]")

    if args.dry_run:
        info("[cyan]--dry-run: stopping here. Nothing sent, nothing recorded.[/cyan]")
        return False
    if args.yes:
        return True

    # A word, not y/N: a bulk send to real guests should take deliberate effort.
    answer = input("Type 'send' to proceed: ").strip().lower()
    return answer == "send"


def deliver(
    config: Config,
    ledger: Ledger,
    args: argparse.Namespace,
    candidates: list[Recipient],
    session: Any,
) -> tuple[list[Recipient], list[Recipient]]:
    """Send one at a time, recording each result before moving on."""
    sent: list[Recipient] = []
    failed: list[Recipient] = []

    for index, recipient in enumerate(candidates, start=1):
        label = f"[{index}/{len(candidates)}] {recipient.name}"
        try:
            message = session.send_text(recipient.digits, recipient.body)
        except WhatsAppError as exc:
            recipient.skip_reason = str(exc)
            failed.append(recipient)
            ledger.record(
                args.event,
                recipient.id,
                recipient.name,
                phone.display(recipient.digits),
                FAILED,
                detail=str(exc),
            )
            error(f"{label} → {phone.display(recipient.digits)}: {exc}")
        except KeyboardInterrupt:
            info("\n[yellow]Interrupted. Progress is saved — re-run to resume.[/yellow]")
            break
        else:
            sent.append(recipient)
            ledger.record(
                args.event,
                recipient.id,
                recipient.name,
                phone.display(recipient.digits),
                SENT,
                message_id=message.message_id,
            )
            info(f"[green]✓[/green] {label} → {phone.display(recipient.digits)}")

        if index == len(candidates):
            break

        # Throttle. Bulk-messaging from a personal number is what gets numbers
        # rate-limited, so the gaps are jittered and batches get a longer pause.
        try:
            if index % config.batch_size == 0:
                info(f"[dim]batch of {config.batch_size} done — pausing "
                     f"{config.batch_pause_seconds:.0f}s[/dim]")
                time.sleep(config.batch_pause_seconds)
            else:
                time.sleep(
                    config.delay_seconds + random.uniform(0, config.jitter_seconds)
                )
        except KeyboardInterrupt:
            info("\n[yellow]Interrupted. Progress is saved — re-run to resume.[/yellow]")
            break

    return sent, failed


def report(
    event: dict[str, Any],
    sent: list[Recipient],
    skipped: list[Recipient],
    failed: list[Recipient] | None = None,
) -> None:
    failed = failed or []
    rows = [[r.name, phone.display(r.digits), SENT, ""] for r in sent]
    rows += [[r.name, phone.display(r.digits), FAILED, r.skip_reason or ""] for r in failed]
    rows += [
        [
            r.name,
            phone.display(r.digits) if r.digits else "-",
            SKIPPED,
            r.skip_reason or "",
        ]
        for r in skipped
    ]
    table(
        f"{couple_of(event)} — result",
        ["Guest", "Phone", "Status", "Detail"],
        rows,
    )
    info(
        f"sent={len(sent)} failed={len(failed)} skipped={len(skipped)}"
    )


# --- entry point ---------------------------------------------------------


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="wa-invite",
        description=(
            "Send rinvite e-invites over WhatsApp from a locally paired account. "
            "Reads guests from the rinvite API and messages the ones whose "
            "channel is 'einvite'."
        ),
    )
    parser.add_argument("--version", action="version", version=f"wa-invite {__version__}")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("login", help="pair this machine with your WhatsApp account").set_defaults(
        func=cmd_login
    )

    events = sub.add_parser("events", help="list your rinvite events")
    events.set_defaults(func=cmd_events)

    guests = sub.add_parser("guests", help="list e-invite guests for an event")
    guests.add_argument("--event", required=True, help="event id")
    guests.add_argument(
        "--all", action="store_true", help="include print-channel guests too"
    )
    guests.set_defaults(func=cmd_guests)

    preview = sub.add_parser(
        "preview", help="render messages without touching WhatsApp"
    )
    preview.add_argument("--event", required=True, help="event id")
    preview.add_argument("--limit", type=int, default=3, help="how many to show")
    preview.add_argument("--only", help="filter by guest name or phone substring")
    preview.set_defaults(func=cmd_preview)

    send = sub.add_parser("send", help="send the e-invites")
    send.add_argument("--event", required=True, help="event id")
    send.add_argument(
        "--dry-run",
        action="store_true",
        help="resolve, render and report — send nothing, record nothing",
    )
    send.add_argument("--limit", type=int, help="send at most this many")
    send.add_argument("--only", help="filter by guest name or phone substring")
    send.add_argument(
        "--resend", action="store_true", help="include guests already marked sent"
    )
    send.add_argument("--yes", action="store_true", help="skip the confirmation prompt")
    send.set_defaults(func=cmd_send)

    status = sub.add_parser("status", help="show the local send ledger")
    status.add_argument("--event", required=True, help="event id")
    status.set_defaults(func=cmd_status)

    return parser


def run(args: argparse.Namespace) -> int:
    try:
        return args.func(args)
    except WaInviteError as exc:
        error(str(exc))
        return exc.exit_code
    except KeyboardInterrupt:
        error("interrupted")
        return 130


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    code = run(args)

    # neonize leaves behind a non-daemon thread that never joins, so a command
    # that touched WhatsApp would hang forever at interpreter shutdown. Every
    # ledger write is committed as it happens and the streams are flushed here,
    # so there is nothing left for normal shutdown to do.
    from wa_invite import whatsapp

    if whatsapp.hard_exit_required():
        sys.stdout.flush()
        sys.stderr.flush()
        os._exit(code)

    return code


if __name__ == "__main__":
    sys.exit(main())
