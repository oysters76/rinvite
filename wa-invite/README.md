# wa-invite

Send rinvite e-invites over WhatsApp from **your own** WhatsApp account, running
locally. Reads guests from the hosted rinvite API, keeps only the ones whose
`channel` is `einvite`, renders a message per guest with their personal invite
link, and sends them one at a time.

The rinvite backend can already send WhatsApp invites, but only through Twilio —
metered, and business-initiated messages need pre-approved templates. This tool
is the free alternative for a one-off wedding. **It changes nothing on the
server; it is a read-only API consumer.**

---

## ⚠️ Read this first

This uses [neonize](https://github.com/krypton-byte/neonize), Python bindings
over Go's `whatsmeow`. It speaks the real WhatsApp Web multi-device protocol —
the same one the desktop app uses — but it is an **unofficial client**.

**Bulk-messaging from a personal number can get that number rate-limited or
banned.** The tool throttles sends, jitters the gaps, pauses between batches and
makes you type `send` to confirm, but the risk cannot be removed. This is
intended for guests who know you and expect the message, not cold contacts.

Start with `--dry-run`, then `--only <your own name>`, before sending to anyone else.

---

## Install

Needs Python ≥3.10 and `libmagic` (neonize's `python-magic` dependency):

```bash
brew install libmagic
```

Then:

```bash
python3 -m venv .venv && .venv/bin/pip install -e '.[dev]'
```

## Configure

Everything comes from `.env` — **nothing is ever prompted for, and a missing
required value aborts the command**:

```bash
cp .env.example .env
```

| Variable | Purpose |
|---|---|
| `RINVITE_API_BASE_URL` | e.g. `https://api.rinvite.link` |
| `RINVITE_TOKEN` | JWT from `POST /auth/login`. Expires after 24 h. |
| `RINVITE_EMAIL` / `RINVITE_PASSWORD` | Used only when `RINVITE_TOKEN` is blank, so the 24 h expiry doesn't force a re-paste. |
| `INVITE_BASE_URL` | **Optional.** Rewrites the origin of every invite link (see below). |
| `WA_DEFAULT_COUNTRY_CODE` | Prefix for bare / leading-zero numbers. `94` = Sri Lanka. |
| `WA_DELAY_SECONDS`, `WA_JITTER_SECONDS` | Gap between sends, plus random jitter. |
| `WA_BATCH_SIZE`, `WA_BATCH_PAUSE_SECONDS` | Longer pause every N messages. |
| `WA_TEMPLATE_FILE` | Message template. Defaults to `templates/whatsapp.txt`. |
| `WA_SESSION_DB`, `WA_LEDGER_DB` | Local state. Both are gitignored. |

### Invite links and reverse proxies

Each guest's `invite_url` comes from the API as `{server INVITE_BASE_URL}/i/{token}`.
If the server hands back an origin your guests cannot reach (a localhost URL, or
an internal host behind a proxy), set `INVITE_BASE_URL` here. The **origin is
replaced and the `/i/{token}` path is preserved byte-for-byte** — the token is
case-sensitive:

```
http://localhost:3000/i/DneQdIEi1m2u  →  https://rinvite.ceykod.com/i/DneQdIEi1m2u
```

Leave it blank to use links exactly as issued. You get a warning if the final
link isn't `https`.

### The message

`templates/whatsapp.txt` is seeded from the server's own wording
(`assets/messages/whatsapp.txt`) so guests get what the Twilio path would have
sent — but it lives here now, so edit it freely. Placeholders:

`{guest_name}` `{couple}` `{bride_name}` `{groom_name}` `{date}` `{time}`
`{hall}` `{venue}` `{rsvp_by}` `{invite_url}` `{max_party_size}` `{poruwa_time}`

`{couple}` follows the event's `precedence` field. A placeholder that isn't in
that list is a **hard error** — a literal `{venue}` reaching a guest is
unrecoverable, so the send refuses to start.

> The one intentional difference from the Rust renderer: single-digit days come
> out as `5 September` rather than the server's space-padded `` 5 September``.

## Use

```bash
.venv/bin/wa-invite login                      # pair once — scan the QR
.venv/bin/wa-invite events                     # find your event id
.venv/bin/wa-invite guests --event <id>        # who will get a message
.venv/bin/wa-invite preview --event <id>       # render messages, no WhatsApp at all
.venv/bin/wa-invite send --event <id> --dry-run
.venv/bin/wa-invite send --event <id> --only "Ravi"   # test on yourself first
.venv/bin/wa-invite send --event <id>
.venv/bin/wa-invite status --event <id>        # what has been sent
```

`send` flags: `--dry-run`, `--limit N`, `--only <name-or-phone substring>`,
`--resend`, `--yes`.

### What gets skipped

Guests are never silently dropped — every one lands in the result table with a
reason: no phone on file, an unparseable number, no WhatsApp account for that
number, or already sent.

### Re-running is safe

`ledger.db` records every send as it happens. A second run skips guests already
sent (`--resend` overrides), and a crashed or `Ctrl-C`'d run resumes exactly
where it stopped. Failures are recorded too, and are retried on the next run.

## Test

```bash
.venv/bin/python -m pytest
```

Everything except the neonize wrapper is pure and tested without a network or a
phone: phone normalisation, link rewriting, template rendering (against the same
fixture as the Rust renderer's test), config validation and ledger behaviour.

## Exit codes

`0` ok · `1` some sends failed · `2` config · `3` API · `4` WhatsApp · `5` template

## Note on process exit

neonize runs its Go event loop on a non-daemon thread that never returns, even
after `stop()`. Any command that opens a WhatsApp session therefore ends with an
explicit `os._exit` after flushing output — otherwise the process would hang
forever at interpreter shutdown. Every ledger write is committed as it happens,
so nothing is lost.
