# Contributing to rinvite

Thanks for your interest in improving rinvite! This guide covers the dev setup,
the architectural rules that keep the codebase clean, and the concrete recipe
for adding a feature. It's aimed at developers comfortable with Rust.

---

## 1. Prerequisites

- **Rust** with edition‑2024 support (**1.85+**; CI uses stable). Install via [rustup](https://rustup.rs).
  - Components: `rustfmt` and `clippy` (`rustup component add rustfmt clippy`).
- **Docker** (optional) — for the Postgres path and to reproduce CI's image build.
- **Postgres** (optional) — only if you want to exercise the SQL adapters; the
  default in‑memory store needs nothing.

No live database is required to build, run, or test.

---

## 2. Getting started

```bash
git clone <your-fork-url> rinvite && cd rinvite

export JWT_SECRET=$(openssl rand -hex 32)   # required to boot
cargo run                                    # in-memory store on :3000

cargo test                                   # everything, no DB needed
```

Run against Postgres locally:

```bash
docker run -d --name rinvite-pg -p 5432:5432 \
  -e POSTGRES_USER=auth -e POSTGRES_PASSWORD=auth -e POSTGRES_DB=auth postgres:16-alpine

export DATABASE_URL=postgres://auth:auth@localhost:5432/auth
cargo run    # migrations run automatically on boot
```

---

## 3. The one rule: dependencies point inward

rinvite is **hexagonal (ports & adapters)**. Before writing code, internalize
the dependency rule — it's what keeps the core testable and swappable:

```
domain  ─◄─  application  ─◄─  adapter / main
(pure)       (use cases)       (axum, sqlx, argon2, printpdf, …)
```

- **`domain/`** — entities (`Event`, `Guest`, `User`), value objects, `DomainError`,
  validation, and the **port traits**. It must not import `axum`, `sqlx`,
  `printpdf`, etc. If you're tempted to, you're in the wrong layer.
- **`application/`** — use‑case logic (`*ServiceImpl`) written **only against port
  traits**. No framework or DB types here either.
- **`adapter/`** — the outside world:
  - `inbound/http/` — axum routes, request/response **DTOs**, the `AuthUser`
    extractor, and the single `DomainError → HTTP status` mapping.
  - `outbound/` — concrete implementations of the outbound ports (argon2, JWT,
    clock, PDF, sender, and the in‑memory/Postgres repositories).
- **`main.rs`** — the composition root; the only place that knows every concrete type.

**Ports live in `domain/port/`:** `inbound.rs` = what the app offers to the
outside (driving), `outbound.rs` = what the app needs from the outside (driven).

---

## 4. Recipe: adding a feature the hexagonal way

Say you want to add "duplicate an event." Work outside‑in through the layers:

1. **Inbound port** (`domain/port/inbound.rs`) — add the use case to the relevant
   trait, e.g. `EventService::duplicate_event(owner, id) -> Event`.
2. **Outbound port** (`domain/port/outbound.rs`) — only if you need something new
   from the outside (a new query, a mailer, …). Add a trait method.
3. **Application** (`application/*_service.rs`) — implement the use case against
   the traits. Reuse the existing ownership gates (`owned_event`, `guest_of`)
   and the `validate_*` helpers. **No axum/sqlx here.**
4. **Outbound adapter(s)** — implement any new outbound method in *both*
   `persistence/events_memory.rs` and `persistence/events_postgres.rs` (keep them
   in sync), plus any other affected adapter.
5. **Inbound adapter** (`inbound/http/…`) — add the DTO(s), the handler, and a
   `.route(...)` line. Map any new `DomainError` variant to a status code in
   `http/mod.rs`.
6. **Wire it** in `main.rs` if new dependencies were introduced.

Steps 1–3 never mention axum or sqlx — that's the whole point.

### Errors

Return a `DomainError` from the core; map it to HTTP in exactly one place
(`ApiError`'s `IntoResponse` in `http/mod.rs`). Never leak internal detail on
5xx. Owner‑scoped resources are reported as `NotFound` (not `Forbidden`) so we
don't confirm the existence of other users' data.

### Database migrations

Migrations are plain SQL in `migrations/`, applied on boot via
`sqlx::migrate!`. Add a new file with the next sequential prefix
(`000N_description.sql`); never edit an already‑released migration. The
in‑memory adapter has no schema, so mirror any behavior a migration implies
(e.g. cascading deletes) in the in‑memory code.

---

## 5. Coding standards

Every change must pass what CI checks:

```bash
cargo fmt --all --check                        # formatting
cargo clippy --all-targets -- -D warnings      # zero warnings
cargo test --all                               # tests green
```

- Match the surrounding style (comment density, naming, idioms).
- Prefer small, focused functions; keep HTTP handlers thin (parse → call port → shape output).
- Keep the in‑memory and Postgres repositories behaviorally identical.

---

## 6. Testing

- **Unit/integration tests** live next to the code in `#[cfg(test)]` modules.
  The in‑memory adapters (`InMemoryEventStore`, `InMemoryUserRepository`) plus
  small fakes (see `application/event_service.rs` tests) let you exercise full
  use cases with no web server or database.
- **Manual PDF checks**: render an invite, then rasterize it to inspect visually,
  e.g. `qlmanage -t -s 1600 -o out invite.pdf` (macOS) or `pdftoppm` (poppler).
- **Postgres path**: spin up the container from §2 and re‑run your flow to
  confirm the SQL adapters and migrations behave.

---

## 7. Commits & pull requests

- Branch off `master`; keep PRs focused and reasonably small.
- Write clear commit messages (imperative mood: "add event duplication").
- In the PR description, explain the **why**, note any new env vars or
  migrations, and confirm `fmt` / `clippy` / `test` pass.
- Update docs when behavior changes: the API table in `README.md`, the env‑var
  table, and the Postman collection (`postman/rinvite.postman_collection.json`).

---

## 8. CI

GitHub Actions (`.github/workflows/ci.yml`) runs on every push/PR:

1. `cargo fmt --all --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --all`
4. `docker build` (validates the image still builds)

Green CI is required to merge.

---

## Where things live (quick map)

| I want to change… | Look in… |
|---|---|
| A business rule / validation | `domain/validation.rs`, `domain/{event,guest}.rs` |
| A use case | `application/*_service.rs` |
| An HTTP route / DTO | `adapter/inbound/http/{auth,events,invites}.rs` |
| Auth / JWT | `adapter/outbound/jwt_issuer.rs`, `http/auth_extractor.rs` |
| Password hashing | `adapter/outbound/argon2_hasher.rs` |
| PDF rendering | `adapter/outbound/pdf.rs`, `assets/pdf-config.json` |
| The e‑invite page | `adapter/inbound/http/html.rs`, `assets/einvite/template.html` |
| Persistence (SQL / in‑memory) | `adapter/outbound/persistence/` |
| Wiring everything together | `main.rs` |

Happy hacking! 💐
