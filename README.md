<div align="center">

# rinvite

**A self-hostable wedding invitation and RSVP service. It is built in Rust with a clean hexagonal architecture.**

Create an event, build a guest list, and invite each person in two ways:
a **printable PDF** on your own card design, or an **animated e-invite web
page** that collects the RSVPs for you.

![Rust](https://img.shields.io/badge/Rust-edition_2024-000000?logo=rust)
![Architecture](https://img.shields.io/badge/architecture-hexagonal-8a7a63)
![Svelte](https://img.shields.io/badge/Svelte-v5-ff3e00?logo=svelte&logoColor=white)
![Auth](https://img.shields.io/badge/auth-argon2_%2B_JWT-b38728)
![License](https://img.shields.io/badge/license-MIT-blue)

</div>

---

## Features

- **Secure auth** — signup and login with argon2id hashing and JWT sessions. The login is timing-safe, so it does not disclose which accounts exist. The server needs a signing secret and stops if the secret is absent.
- **Email verification** — signup sends a verification link by email. The server refuses login until the user confirms the address. The user can request the link again with one click.
- **Subscription plans** — Free, Pro, and Max tiers, each with its own usage limits. When a user reaches a limit, the app opens a dialog to contact the owner and to request an upgrade.
- **Events and guests** — full create, read, update, and delete for wedding events and their guest lists. Each user sees only their own data.
- **PDF invitations** — put personalized text onto *your* card image with configurable fonts, positions, and colors. The default page size is A5.
- **Animated e-invites** — a self-contained HTML invitation with a built-in RSVP form. The server sends one page for each guest. The page shows opening gates, falling petals, and gold styling.
- **RSVP collection** — each guest replies through a unique link. The server checks the party size against a per-guest limit and against the RSVP deadline.
- **Bulk actions** — merge all printed invitations into one PDF to download, or send all e-invites in sequence and get a delivery report for each guest.
- **Organizer dashboard** — an optional web UI (SvelteKit and shadcn-svelte) in [`web/`](web/). It manages events, guests, RSVP status, CSV import, and bulk send and print. It uses the same API.
- **Hexagonal architecture** — the domain has no framework or database dependencies. You can replace Postgres with the in-memory store, or the no-op WhatsApp sender with a real one, without a change to the core.
- **Runs anywhere** — use the in-memory store for local development with no setup, or Postgres for production. The repo includes a Dockerfile, a docker-compose file, and CI.

---

## Tech stack

| Concern | Choice |
|---|---|
| Language | Rust (edition 2024) |
| HTTP | [axum](https://github.com/tokio-rs/axum) 0.8 + [tower-http](https://github.com/tower-rs/tower-http) |
| Async runtime | [tokio](https://tokio.rs) |
| Database | [sqlx](https://github.com/launchbadge/sqlx) 0.9 (Postgres, plain SQL, rustls) |
| Auth | [argon2](https://crates.io/crates/argon2) + [jsonwebtoken](https://crates.io/crates/jsonwebtoken) (HS256) |
| PDF | [printpdf](https://crates.io/crates/printpdf) + [ttf-parser](https://crates.io/crates/ttf-parser) |
| Frontend *(optional)* | [SvelteKit](https://svelte.dev/) + [shadcn-svelte](https://shadcn-svelte.com) + Tailwind, in [`web/`](web/) |

There is no ORM and no macro magic. The code uses only ports, adapters, and plain SQL.

---

## Quick start (no setup)

You need a recent Rust toolchain (edition 2024, that is **Rust 1.85 or later**). You do not need a database. The server uses an in-memory store if you do not set one.

```bash
git clone <your-fork-url> rinvite && cd rinvite

# The signing key is REQUIRED (min 32 bytes). Generate one:
export JWT_SECRET=$(openssl rand -hex 32)

cargo run
# → Listening on http://0.0.0.0:3000
```

Then do the full sequence with `curl`:

```bash
BASE=http://localhost:3000

# 1) Sign up → emails a verification link (logged to the console in local dev)
curl -s -X POST $BASE/auth/signup \
  -H 'content-type: application/json' \
  -d '{"email":"host@example.com","password":"hunter2!"}' | jq

# 1a) Verify using the token from that link, then log in for a JWT
curl -s -X POST $BASE/auth/verify \
  -H 'content-type: application/json' -d '{"token":"<token-from-email>"}'
TOKEN=$(curl -s -X POST $BASE/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"host@example.com","password":"hunter2!"}' | jq -r .token)

# 2) Create a wedding event
EVENT=$(curl -s -X POST $BASE/events -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' -d '{
    "bride_name":"Hansika","bride_family_name":"Jaliyagoda",
    "groom_name":"Chirath","groom_family_name":"Nishshanka",
    "event_date":"2026-09-25","start_time":"10:00:00","end_time":"15:00:00",
    "hall_name":"the Kings Ballroom","venue_name":"Peradeniya Rest House, Kandy",
    "rsvp_by":"2026-08-20"}' | jq -r .id)

# 3) Add an e-invite guest → response includes a shareable invite_url
curl -s -X POST $BASE/events/$EVENT/guests -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"Mr Dhammika & family","channel":"einvite","email":"d@example.com","max_party_size":2}' | jq

# 4) Open the invite_url in a browser — or download a print PDF:
curl -s -X POST $BASE/events/$EVENT/guests -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"Aunty Kamala","channel":"print","max_party_size":1}' | jq
curl -s "$BASE/events/$EVENT/invites/print.pdf" -H "authorization: Bearer $TOKEN" -o invites.pdf
```

> Do you prefer Postman? Import [`postman/rinvite.postman_collection.json`](postman/rinvite.postman_collection.json). It contains every endpoint, and the requests chain automatically.

---

## Web dashboard (optional UI)

If you prefer to click instead of to use `curl`, use the **SvelteKit and
shadcn-svelte** organizer dashboard in [`web/`](web/). It is a static
single-page app that uses this API. It covers the full workflow: create, edit,
and delete events; manage the guest list (single add, quick-add, and **CSV
import**); search, filter, and sort; per-guest and **bulk send and download
PDF**; move guests between channels; and a live RSVP summary.

```bash
# 1) run the API (see Quick start above)
export JWT_SECRET=$(openssl rand -hex 32)
cargo run                       # → http://localhost:3000

# 2) in another terminal, run the dashboard
cd web
npm install
npm run dev                     # → http://localhost:5173
```

Open **http://localhost:5173**, create an account, and start. The dashboard
reads the API base URL from `VITE_API_BASE_URL` (the default is
`http://localhost:3000`). Build a static bundle with `npm run build` and deploy
it to any static host. Then set `VITE_API_BASE_URL` to your API, and set the
backend's `CORS_ALLOWED_ORIGINS` to the dashboard's origin. See
[`web/README.md`](web/README.md) for the details.

> CORS works in development with no setup. The API allows any origin until you
> set `CORS_ALLOWED_ORIGINS`.

---

## Configuration

You set all configuration with environment variables:

| Variable | Required | Default | Purpose |
|---|:---:|---|---|
| `JWT_SECRET` | ✅ | — | JWT signing key; **must be ≥ 32 bytes**. The server refuses to start without it. |
| `DATABASE_URL` | | *(in‑memory)* | Postgres DSN, e.g. `postgres://user:pass@host:5432/db`. Unset → in‑memory store. |
| `PUBLIC_BASE_URL` | | `http://localhost:3000` | Base URL of this API. Invite links use it when `INVITE_BASE_URL` is not set. |
| `INVITE_BASE_URL` | | *(`PUBLIC_BASE_URL`)* | Base URL that guests use for invite links, e.g. `https://rinvite.ceykod.com`. Set it to a reverse proxy in front of the `/i/*` routes. |
| `FRONTEND_BASE_URL` | | `http://localhost:5173` | Base URL of the web app. The email‑verification link uses it. |
| `CORS_ALLOWED_ORIGINS` | | *(any origin)* | Comma‑separated allowlist for a browser frontend. Unset allows any origin (safe — auth is Bearer‑token, no cookies). |
| `PDF_CONFIG` | | *(plain page)* | Path to the PDF layout JSON (see [Customizing the PDF](#customizing-the-pdf)). Unset → a plain text‑only fallback. |
| `EINVITE_TEMPLATE` | | *(embedded)* | Path to a custom e‑invite HTML template. Unset → the built‑in [`assets/einvite/template.html`](assets/einvite/template.html). |
| `RESEND_API_KEY` | | *(log‑only)* | [Resend](https://resend.com) API key. With `EMAIL_FROM`, e‑invite emails are actually sent; unset → logged to the console. |
| `EMAIL_FROM` | | — | Sender for emails, e.g. `Rinvite <invites@your-domain>`. |
| `TWILIO_ACCOUNT_SID` / `TWILIO_AUTH_TOKEN` / `TWILIO_WHATSAPP_FROM` | | *(log‑only)* | [Twilio WhatsApp](https://www.twilio.com/docs/whatsapp) credentials + sender (E.164, e.g. `+14155238886`). All three set → WhatsApp messages are sent; otherwise logged. |
| `TWILIO_CONTENT_SID` | | *(freeform)* | A Meta‑approved template's ContentSid. **Required for production** business‑initiated WhatsApp (freeform text only works in Twilio's sandbox / a 24h session window). |
| `EMAIL_TEMPLATE_HTML` / `EMAIL_TEMPLATE_TEXT` / `EMAIL_SUBJECT` / `WHATSAPP_TEMPLATE` | | *(embedded)* | Paths to override the built‑in message templates in [`assets/messages/`](assets/messages/). |
| `BUSINESS_CONTACT_EMAIL` | | `hello@example.com` | Contact address shown to users in the "limit reached" dialog. |
| `UPGRADE_NOTIFY_EMAIL` | | *(= contact)* | Recipient of upgrade‑request notifications (the app owner). |
| `VERIFY_EMAIL_TEMPLATE_*` / `UPGRADE_EMAIL_TEMPLATE_*` (`HTML`/`TEXT`/`SUBJECT`) | | *(embedded)* | Paths to override the account‑lifecycle email templates in [`assets/messages/`](assets/messages/). |

The server listens on **port 3000**.

### Plans, verification, and upgrade requests

A new account starts unverified. Signup sends a verification link
(`{FRONTEND_BASE_URL}/verify?token=…`), and **the server refuses login until the
user confirms the email**. The user can request the link again from the signup
or login screen.

Each account has a plan. The server applies the plan on create-event and on
add-guest:

| Plan | Events | Guests per event |
|---|---|---|
| Free | 1 | 10 |
| Pro | 5 | 100 |
| Max | unlimited | unlimited |

If a user reaches a limit, the server returns **HTTP 402**. The dashboard then
opens a dialog with the `BUSINESS_CONTACT_EMAIL` and a **Request an upgrade**
button. The button sends an email to `UPGRADE_NOTIFY_EMAIL`, from the
customizable [`upgrade-request.*`](assets/messages/) templates. There is no
self-serve billing. An operator changes the `plan` value in the row
(`free`, `pro`, or `max`) to upgrade the account. The verification and upgrade
emails use the same log-only fallback as the rest of the app, so all functions
work in local development without a Resend account.

### E-invite delivery (WhatsApp and email)

When you send a guest their e-invite link, Rinvite selects the channel
automatically. **A guest with a phone number gets WhatsApp**; a guest without
one gets **email**. The server builds the message from the editable templates in
[`assets/messages/`](assets/messages/) (`email.html`, `email.txt`,
`email-subject.txt`, and `whatsapp.txt`). Each template accepts the placeholders
`{guest_name} {couple} {bride_name} {groom_name} {date} {time} {venue} {hall}
{rsvp_by} {invite_url}`. The links are short: `{INVITE_BASE_URL}/i/<token>`, for
example `https://rinvite.ceykod.com/i/4VHmLjQrcrav`. The server also answers
these paths on its own host, thus links that you sent before stay usable.

If you do not set provider keys, the server writes the message (with the link)
to the server console instead. The dashboard's **Send** function continues to
operate in local development without a Resend or Twilio account.

---

## Self-hosting

### Option A — Docker (single container, in-memory)

```bash
docker build -t rinvite .
docker run -p 3000:3000 -e JWT_SECRET=$(openssl rand -hex 32) rinvite
```

### Option B — Docker Compose (app and Postgres, recommended)

```bash
cp .env.example .env      # then set JWT_SECRET (openssl rand -hex 32)
docker compose up --build
```

Compose starts Postgres with a persistent volume and waits until Postgres is
healthy before it starts the app. The database migrations run automatically at
boot.

### Option C — DigitalOcean with Terraform (backend, frontend, DB, and custom domain)

One command deploys the full stack (the API, the static web app, managed
Postgres, DNS, and automatic TLS on your custom domain) to DigitalOcean App
Platform. The deploy is reproducible:

```bash
cd deploy/terraform
cp terraform.tfvars.example terraform.tfvars   # set do_token + root_domain
terraform init && terraform apply
```

See [deploy/terraform/README.md](deploy/terraform/README.md) for the full runbook.

### Production checklist

- **Set a strong `JWT_SECRET`** (32 or more random bytes) and keep it out of version control.
- **Set `DATABASE_URL`**. If you do not, the app uses the in-memory store and loses all data at restart.
- **Set `PUBLIC_BASE_URL`** to your real public URL, so the invite links point to the correct place.
- **Set `INVITE_BASE_URL`** if a reverse proxy gives the invite links a different host. The proxy must send the `/i/` paths to this server. It must also set a `Host` header that the server's platform accepts.
- **Set `CORS_ALLOWED_ORIGINS`** to your frontend's origin or origins if you run a separate SPA.
- Put the app behind a TLS-terminating reverse proxy (nginx, Caddy, or Traefik).
- Mount your own `assets/` (card image, fonts, and templates) and point `PDF_CONFIG` and `EINVITE_TEMPLATE` at them. This gives the invitations your own brand without a rebuild.

---

## API reference

**Public**

| Method | Path | Description |
|---|---|---|
| `POST` | `/auth/signup` | Create an account → `{ verification_required }` (emails a link) |
| `POST` | `/auth/verify` | Confirm an email `{ token }` → `204` |
| `POST` | `/auth/resend-verification` | Re‑send the verification email `{ email }` → `204` |
| `POST` | `/auth/login` | Log in → `{ token }` (`403` until the email is verified) |
| `GET` | `/config` | Public client config `{ contact_email }` |
| `GET` | `/invite/{token}` | The guest's e‑invite web page (HTML) |
| `POST` | `/invite/{token}/rsvp` | Submit an RSVP `{ attending, party_size }` |

**Authenticated** (`Authorization: Bearer <token>`)

| Method | Path | Description |
|---|---|---|
| `GET` | `/auth/me` | The current user `{ id, email, plan, email_verified }` |
| `POST` | `/billing/upgrade-request` | Email the owner requesting a plan upgrade → `204` |
| `POST` · `GET` | `/events` | Create / list your events |
| `GET` · `PATCH` · `DELETE` | `/events/{id}` | Read / partial‑update / delete an event |
| `POST` · `GET` | `/events/{id}/guests` | Add / list guests |
| `GET` · `PATCH` · `DELETE` | `/events/{id}/guests/{gid}` | Read / update / delete a guest |
| `GET` | `/events/{id}/guests/{gid}/invite.pdf` | One guest's printable PDF |
| `POST` | `/events/{id}/guests/{gid}/send` | Send one e‑invite (via the configured sender) |
| `GET` | `/events/{id}/invites/print.pdf` | **Bulk:** merged PDF of all print‑channel guests |
| `POST` | `/events/{id}/invites/send` | **Bulk:** sequentially send all e‑invites → report |

The errors are consistent JSON: `{ "error": "message" }` with a correct status code (`400/401/404/409/422/500`).

---

## Customizing the PDF

The PDF renderer puts text onto a **base card image** with a JSON layout that
you control ([`assets/pdf-config.json`](assets/pdf-config.json)):

```jsonc
{
  "template_image": "assets/templates/floral-gold.png",
  "page_mm": [148, 210],          // A5; omit to size the page to the image
  "dpi": 300,
  "fonts": {                      // any number of named TTFs
    "serif":  "assets/fonts/EBGaramond-Regular.ttf",
    "script": "assets/fonts/GreatVibes-Regular.ttf",
    "caps":   "assets/fonts/Cinzel-Variable.ttf"
  },
  "elements": [                   // positioned text, each with its own font/size/color
    { "template": "{bride_name}", "font": "script", "size": 40,
      "x_mm": 74, "y_mm": 103, "align": "center", "color": [0.72,0.56,0.22] },
    { "template": "From {start_time} to {end_time}", "font": "serif", "size": 11,
      "x_mm": 74, "y_mm": 56, "align": "center" }
  ]
}
```

The server fills placeholders such as `{bride_name}`, `{guest_name}`,
`{day_ordinal}`, `{month}`, `{start_time}`, `{rsvp_by}`, and `{venue_name}` for
each guest. Point `PDF_CONFIG` at your file and use your own image and fonts. A
recompile is not necessary.

## Customizing the e-invite

The animated invitation is in
[`assets/einvite/template.html`](assets/einvite/template.html). The server puts
each guest's data into the page as JSON, and the page renders itself. Edit the
file (or set `EINVITE_TEMPLATE` to a copy) to change the style. The server
escapes the guest and event values against XSS, so your markup stays
declarative.

---

## Architecture

rinvite uses a **hexagonal (ports and adapters)** architecture. The dependency
rule points inward: `domain` depends on nothing, `application` depends only on
`domain`, and `adapter` and `main` depend on both.

```
src/
├── domain/                     # the hexagon — pure business types, no axum/sqlx
│   ├── event.rs · guest.rs · model.rs · error.rs · validation.rs
│   └── port/
│       ├── inbound.rs          # what the app offers (AuthService, EventService, …)
│       └── outbound.rs         # what the app needs (repositories, hasher, sender, …)
├── application/                # use-case logic, implemented against ports only
│   ├── auth_service.rs · event_service.rs · invite_service.rs
├── adapter/
│   ├── inbound/http/           # axum: routes, DTOs, auth extractor, error mapping
│   └── outbound/               # argon2, jwt, clock, pdf, sender, persistence/
│       └── persistence/        # in-memory + Postgres implementations
└── main.rs                     # composition root — wires adapters to ports
```

The result: you can test the full core without a web server or a database, and a
change of implementation (for example, a real e-invite sender) does not touch
the business logic.

---

## Testing

```bash
cargo test           # unit + integration tests (in-memory adapters, no DB needed)
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

---

## Roadmap

These are the planned enhancements. Contributions are welcome.

- WhatsApp opt-out (STOP) handling, a per-guest `sent_at` value, and a retry queue for very large batches
- A JSON form of `GET /i/{token}` for frontends that render their own invite
- RSVP summary and aggregates, pagination, and a `/health` endpoint
- Rate limiting on auth, structured logging, and an OpenAPI spec
- JWT refresh tokens

---

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the
development setup, the architecture rules to follow, and the steps to add a
feature the hexagonal way.

## License

Released under the **MIT License**. See [LICENSE](LICENSE).
