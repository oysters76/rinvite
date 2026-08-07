"""Message rendering.

The template lives in this project (templates/whatsapp.txt, overridable via
WA_TEMPLATE_FILE) rather than on the server. The placeholder set and the two
formatting helpers mirror the Rust renderer in
`src/adapter/outbound/message/mod.rs`, so a template file works unchanged in
either place. No HTML escaping — WhatsApp bodies are plain text.
"""

from __future__ import annotations

import re
from datetime import date, datetime, time
from pathlib import Path
from typing import Any

from wa_invite.errors import TemplateError

PLACEHOLDER_RE = re.compile(r"\{([a-z_]+)\}")


def load_template(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise TemplateError(f"cannot read template {path}: {exc}") from exc


def ordinal(day: int) -> str:
    """1st, 2nd, 3rd, 4th … 11th, 12th, 13th — matches the Rust `ordinal`."""
    if day % 100 in (11, 12, 13):
        suffix = "th"
    else:
        suffix = {1: "st", 2: "nd", 3: "rd"}.get(day % 10, "th")
    return f"{day}{suffix}"


def fmt_time(value: time) -> str:
    """12-hour clock, e.g. `10:00 AM`, `12:30 PM` — matches the Rust `fmt_time`."""
    hour24 = value.hour
    if hour24 == 0:
        hour12, meridiem = 12, "AM"
    elif hour24 < 12:
        hour12, meridiem = hour24, "AM"
    elif hour24 == 12:
        hour12, meridiem = 12, "PM"
    else:
        hour12, meridiem = hour24 - 12, "PM"
    return f"{hour12}:{value.minute:02d} {meridiem}"


def parse_date(value: str) -> date:
    return datetime.strptime(value, "%Y-%m-%d").date()


def parse_time(value: str) -> time:
    # The API serialises NaiveTime as HH:MM:SS, but tolerate HH:MM.
    for fmt in ("%H:%M:%S", "%H:%M"):
        try:
            return datetime.strptime(value, fmt).time()
        except ValueError:
            continue
    raise TemplateError(f"cannot parse time {value!r}")


def ordered_names(event: dict[str, Any]) -> tuple[str, str]:
    """Bride/groom in the event's configured precedence order."""
    bride, groom = event["bride_name"], event["groom_name"]
    return (bride, groom) if event.get("precedence") == "bride" else (groom, bride)


def build_vars(
    event: dict[str, Any], guest: dict[str, Any], invite_url: str
) -> dict[str, str]:
    first, second = ordered_names(event)
    event_date = parse_date(event["event_date"])
    rsvp_by = parse_date(event["rsvp_by"])
    start = fmt_time(parse_time(event["start_time"]))
    end = fmt_time(parse_time(event["end_time"]))

    variables = {
        "guest_name": guest["name"],
        "couple": f"{first} & {second}",
        "bride_name": event["bride_name"],
        "groom_name": event["groom_name"],
        # %-d is POSIX; the Rust side uses %e, which is space-padded then used
        # in a context where the padding is invisible.
        "date": f"{event_date.strftime('%A')}, {event_date.day} "
        f"{event_date.strftime('%B %Y')}",
        "time": f"{start} to {end}",
        "venue": event["venue_name"],
        "hall": event["hall_name"],
        "rsvp_by": f"{ordinal(rsvp_by.day)} {rsvp_by.strftime('%B')}",
        "invite_url": invite_url,
        "max_party_size": str(guest.get("max_party_size", "")),
    }

    poruwa = event.get("poruwa_ceremony_time")
    variables["poruwa_time"] = fmt_time(parse_time(poruwa)) if poruwa else ""

    return variables


def render(
    template: str, event: dict[str, Any], guest: dict[str, Any], invite_url: str
) -> str:
    """Fill `{key}` placeholders; an unknown one is a hard error.

    A typo'd placeholder reaching a guest is unrecoverable, so we refuse to
    send rather than deliver a message with a literal `{venue}` in it.
    """
    variables = build_vars(event, guest, invite_url)
    body = template
    for key, value in variables.items():
        body = body.replace(f"{{{key}}}", value)

    leftover = sorted(set(PLACEHOLDER_RE.findall(body)))
    if leftover:
        known = ", ".join(sorted(variables))
        raise TemplateError(
            f"unknown placeholder(s) in template: "
            f"{', '.join('{' + k + '}' for k in leftover)}. Available: {known}"
        )
    return body.strip()
