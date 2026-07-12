<div align="center">

# 💐 rinvite

**A self-hostable wedding invitation & RSVP service — built in Rust with clean hexagonal architecture.**

Create an event, build a guest list, and invite everyone two ways:
a **printable PDF** rendered onto your own card design, or a beautiful
**animated e-invite web page** that collects RSVPs for you.

![Rust](https://img.shields.io/badge/Rust-edition_2024-000000?logo=rust)
![Architecture](https://img.shields.io/badge/architecture-hexagonal-8a7a63)
![Auth](https://img.shields.io/badge/auth-argon2_%2B_JWT-b38728)
![License](https://img.shields.io/badge/license-MIT-blue)

</div>

---

## ✨ Features

- 🔐 **Secure auth** — signup/login with argon2id hashing, JWT sessions, timing‑safe login (no account enumeration), and a fail‑fast required signing secret.
- 💒 **Events & guests** — full CRUD for wedding events and their guest lists, scoped so a user only ever sees their own data.
- 🖨️ **PDF invitations** — overlay personalized text onto *your* card image with configurable fonts, positions, and colors. Sized to A5 out of the box.
- 🌸 **Animated e‑invites** — a gorgeous, self‑contained HTML invitation (opening gates, falling petals, gold styling) served per‑guest, with a built‑in RSVP form.
- ✅ **RSVP collection** — guests respond via their unique link; party size is validated against a per‑guest cap and the RSVP deadline.
- 📦 **Bulk actions** — merge every printed invite into one PDF to download, or sequentially send every e‑invite and get a per‑guest delivery report.
- 🖥️ **Organizer dashboard** — an optional web UI (SvelteKit + shadcn‑svelte) in [`web/`](web/) to manage events, guests, RSVP status, CSV import, and bulk send/print — talking to the same API.
- 🧩 **Hexagonal architecture** — the domain has zero framework/database dependencies; swap Postgres for in‑memory (or a real WhatsApp sender for the no‑op one) without touching the core.
- 🐘 **Runs anywhere** — in‑memory for zero‑setup local dev, or Postgres for production. Ships with a Dockerfile, docker‑compose, and CI.

---

## 🧰 Tech stack

| Concern | Choice |
|---|---|
| Language | Rust (edition 2024) |
| HTTP | [axum](https://github.com/tokio-rs/axum) 0.8 + [tower-http](https://github.com/tower-rs/tower-http) |
| Async runtime | [tokio](https://tokio.rs) |
| Database | [sqlx](https://github.com/launchbadge/sqlx) 0.9 (Postgres, plain SQL, rustls) |
| Auth | [argon2](https://crates.io/crates/argon2) + [jsonwebtoken](https://crates.io/crates/jsonwebtoken) (HS256) |
| PDF | [printpdf](https://crates.io/crates/printpdf) + [ttf-parser](https://crates.io/crates/ttf-parser) |
| Frontend *(optional)* | [SvelteKit](https://svelte.dev/) + [shadcn-svelte](https://shadcn-svelte.com) + Tailwind, in [`web/`](web/) |

No ORM, no macro magic — just ports, adapters, and plain SQL.

---

## 🚀 Quick start (zero setup)

You need a recent Rust toolchain (edition 2024, i.e. **Rust 1.85+**). No database required — it falls back to an in‑memory store.

```bash
git clone <your-fork-url> rinvite && cd rinvite

# The signing key is REQUIRED (min 32 bytes). Generate one:
export JWT_SECRET=$(openssl rand -hex 32)

cargo run
# → Listening on http://0.0.0.0:3000
```

Then walk the whole flow with `curl`:

```bash
BASE=http://localhost:3000

# 1) Sign up → returns a JWT
TOKEN=$(curl -s -X POST $BASE/auth/signup \
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

> 💡 Prefer Postman? Import [`postman/rinvite.postman_collection.json`](postman/rinvite.postman_collection.json) — every endpoint is there and the requests chain automatically.

---

## 🖥️ Web dashboard (optional UI)

Rather click than curl? A **SvelteKit + shadcn‑svelte** organizer dashboard lives
in [`web/`](web/) — a static single‑page app that talks to this API. It covers
the whole workflow: create/edit/delete events, manage the guest list (single add,
quick‑add, **CSV import**), search / filter / sort, per‑guest and **bulk send /
download PDF**, move guests between channels, and a live RSVP summary — in a
clean, minimalist UI.

```bash
# 1) run the API (see Quick start above)
export JWT_SECRET=$(openssl rand -hex 32)
cargo run                       # → http://localhost:3000

# 2) in another terminal, run the dashboard
cd web
npm install
npm run dev                     # → http://localhost:5173
```

Open **http://localhost:5173**, create an account, and go. The dashboard reads
the API base URL from `VITE_API_BASE_URL` (defaults to `http://localhost:3000`).
Build a static bundle with `npm run build` and deploy it to any static host,
then set `VITE_API_BASE_URL` to your API and the backend's `CORS_ALLOWED_ORIGINS`
to the dashboard's origin. See [`web/README.md`](web/README.md) for details.

> CORS just works in dev — the API allows any origin unless you set
> `CORS_ALLOWED_ORIGINS`.

---

## ⚙️ Configuration

All configuration is via environment variables:

| Variable | Required | Default | Purpose |
|---|:---:|---|---|
| `JWT_SECRET` | ✅ | — | JWT signing key; **must be ≥ 32 bytes**. The server refuses to start without it. |
| `DATABASE_URL` | | *(in‑memory)* | Postgres DSN, e.g. `postgres://user:pass@host:5432/db`. Unset → in‑memory store. |
| `PUBLIC_BASE_URL` | | `http://localhost:3000` | Base URL used to build shareable invite links. |
| `CORS_ALLOWED_ORIGINS` | | *(any origin)* | Comma‑separated allowlist for a browser frontend. Unset allows any origin (safe — auth is Bearer‑token, no cookies). |
| `PDF_CONFIG` | | *(plain page)* | Path to the PDF layout JSON (see [Customizing the PDF](#-customizing-the-pdf)). Unset → a plain text‑only fallback. |
| `EINVITE_TEMPLATE` | | *(embedded)* | Path to a custom e‑invite HTML template. Unset → the built‑in [`assets/einvite/template.html`](assets/einvite/template.html). |

The server listens on **port 3000**.

---

## 🏠 Self-hosting

### Option A — Docker (single container, in‑memory)

```bash
docker build -t rinvite .
docker run -p 3000:3000 -e JWT_SECRET=$(openssl rand -hex 32) rinvite
```

### Option B — Docker Compose (app + Postgres, recommended)

```bash
cp .env.example .env      # then set JWT_SECRET (openssl rand -hex 32)
docker compose up --build
```

Compose brings up Postgres with a persistent volume and waits for it to be
healthy before starting the app. Database migrations run automatically on boot.

### Production checklist

- **Set a strong `JWT_SECRET`** (32+ random bytes) and keep it out of version control.
- **Set `DATABASE_URL`** — otherwise the app silently uses the in‑memory store and loses all data on restart.
- **Set `PUBLIC_BASE_URL`** to your real public URL so invite links point to the right place.
- **Set `CORS_ALLOWED_ORIGINS`** to your frontend's origin(s) if you run a separate SPA.
- Put it behind a TLS‑terminating reverse proxy (nginx, Caddy, Traefik).
- Mount your own `assets/` (card image, fonts, templates) and point `PDF_CONFIG` / `EINVITE_TEMPLATE` at them to fully brand the invitations without rebuilding.

---

## 📡 API reference

**Public**

| Method | Path | Description |
|---|---|---|
| `POST` | `/auth/signup` | Create an account → `{ token }` |
| `POST` | `/auth/login` | Log in → `{ token }` |
| `GET` | `/invite/{token}` | The guest's e‑invite web page (HTML) |
| `POST` | `/invite/{token}/rsvp` | Submit an RSVP `{ attending, party_size }` |

**Authenticated** (`Authorization: Bearer <token>`)

| Method | Path | Description |
|---|---|---|
| `GET` | `/auth/me` | The current user `{ id, email }` |
| `POST` · `GET` | `/events` | Create / list your events |
| `GET` · `PATCH` · `DELETE` | `/events/{id}` | Read / partial‑update / delete an event |
| `POST` · `GET` | `/events/{id}/guests` | Add / list guests |
| `GET` · `PATCH` · `DELETE` | `/events/{id}/guests/{gid}` | Read / update / delete a guest |
| `GET` | `/events/{id}/guests/{gid}/invite.pdf` | One guest's printable PDF |
| `POST` | `/events/{id}/guests/{gid}/send` | Send one e‑invite (via the configured sender) |
| `GET` | `/events/{id}/invites/print.pdf` | **Bulk:** merged PDF of all print‑channel guests |
| `POST` | `/events/{id}/invites/send` | **Bulk:** sequentially send all e‑invites → report |

Errors are consistent JSON: `{ "error": "message" }` with a meaningful status code (`400/401/404/409/422/500`).

---

## 🎨 Customizing the PDF

The PDF renderer overlays text onto a **base card image** using a JSON layout
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

Placeholders like `{bride_name}`, `{guest_name}`, `{day_ordinal}`, `{month}`,
`{start_time}`, `{rsvp_by}`, `{venue_name}` are filled per guest. Point
`PDF_CONFIG` at your file and swap in your own image/fonts — no recompile needed.

## 🌸 Customizing the e-invite

The animated invitation lives in [`assets/einvite/template.html`](assets/einvite/template.html).
The server injects each guest's data as JSON and the page renders itself. Edit
the file (or set `EINVITE_TEMPLATE` to a copy) to restyle — guest/event values
are injected safely (XSS‑escaped), so your markup stays declarative.

---

## 🧭 Architecture

rinvite follows **hexagonal (ports & adapters)** architecture. The dependency
rule points inward: `domain` depends on nothing, `application` depends only on
`domain`, and `adapter` / `main` depend on both.

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

The payoff: the entire core is testable without a web server or a database, and
swapping an implementation (e.g. a real e‑invite sender) never touches business logic.

---

## ✅ Testing

```bash
cargo test           # unit + integration tests (in-memory adapters, no DB needed)
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

---

## 🗺️ Roadmap

Planned enhancements (contributions welcome!):

- Real e‑invite delivery adapters (WhatsApp / email) behind the existing `InviteSender` port
- JSON variant of `GET /invite/{token}` for frontends that render their own invite
- RSVP summary/aggregates, pagination, and a `/health` endpoint
- Rate limiting on auth, structured logging, and an OpenAPI spec
- JWT refresh tokens

---

## 🤝 Contributing

Contributions are very welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the
dev setup, the architecture rules to follow, and how to add a feature the
hexagonal way.

## 📄 License

Released under the **MIT License** — see [LICENSE](LICENSE).
