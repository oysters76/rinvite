# Contributing to rinvite

Thank you for your interest in rinvite. This guide covers the development setup,
the architecture rules that keep the codebase clean, and the steps to add a
feature. It is for developers who know Rust.

---

## 1. Prerequisites

- **Rust** with edition-2024 support (**1.85 or later**; CI uses stable). Install it with [rustup](https://rustup.rs).
  - Components: `rustfmt` and `clippy` (`rustup component add rustfmt clippy`).
- **Docker** (optional) — for the Postgres path and to reproduce the CI image build.
- **Postgres** (optional) — only if you want to test the SQL adapters. The
  default in-memory store needs nothing.

You do not need a live database to build, run, or test.

---

## 2. Getting started

```bash
git clone <your-fork-url> rinvite && cd rinvite

export JWT_SECRET=$(openssl rand -hex 32)   # required to boot
cargo run                                    # in-memory store on :3000

cargo test                                   # everything, no DB needed
```

Run against Postgres on your machine:

```bash
docker run -d --name rinvite-pg -p 5432:5432 \
  -e POSTGRES_USER=auth -e POSTGRES_PASSWORD=auth -e POSTGRES_DB=auth postgres:16-alpine

export DATABASE_URL=postgres://auth:auth@localhost:5432/auth
cargo run    # migrations run automatically on boot
```

---

## 3. The one rule: dependencies point inward

rinvite is **hexagonal (ports and adapters)**. Before you write code, learn the
dependency rule. It keeps the core testable and replaceable:

```
domain  ─◄─  application  ─◄─  adapter / main
(pure)       (use cases)       (axum, sqlx, argon2, printpdf, …)
```

- **`domain/`** — the entities (`Event`, `Guest`, `User`), the value objects, `DomainError`,
  the validation, and the **port traits**. It must not import `axum`, `sqlx`,
  `printpdf`, or similar crates. If you want to, you are in the wrong layer.
- **`application/`** — the use-case logic (`*ServiceImpl`), written **only against the port
  traits**. Do not use framework or DB types here.
- **`adapter/`** — the outside world:
  - `inbound/http/` — the axum routes, the request and response **DTOs**, the `AuthUser`
    extractor, and the single `DomainError → HTTP status` mapping.
  - `outbound/` — the concrete implementations of the outbound ports (argon2, JWT,
    clock, PDF, sender, and the in-memory and Postgres repositories).
- **`main.rs`** — the composition root. It is the only place that knows every concrete type.

**The ports are in `domain/port/`.** `inbound.rs` is what the app offers to the
outside (driving). `outbound.rs` is what the app needs from the outside (driven).

---

## 4. Steps: add a feature the hexagonal way

For example, you want to add "duplicate an event." Work from the inside outward
through the layers:

1. **Inbound port** (`domain/port/inbound.rs`) — add the use case to the correct
   trait, for example `EventService::duplicate_event(owner, id) -> Event`.
2. **Outbound port** (`domain/port/outbound.rs`) — only if you need something new
   from the outside (a new query, a mailer, and so on). Add a trait method.
3. **Application** (`application/*_service.rs`) — implement the use case against
   the traits. Reuse the ownership gates (`owned_event`, `guest_of`)
   and the `validate_*` helpers. **Do not use axum or sqlx here.**
4. **Outbound adapters** — implement each new outbound method in *both*
   `persistence/events_memory.rs` and `persistence/events_postgres.rs` (keep them
   in sync), and in any other adapter that the change affects.
5. **Inbound adapter** (`inbound/http/…`) — add the DTOs, the handler, and a
   `.route(...)` line. Map each new `DomainError` variant to a status code in
   `http/mod.rs`.
6. **Wire it** in `main.rs` if the change introduces new dependencies.

Steps 1 to 3 never mention axum or sqlx. That is the point.

### Errors

Return a `DomainError` from the core. Map it to HTTP in exactly one place
(the `IntoResponse` for `ApiError` in `http/mod.rs`). Never disclose internal
detail in a 5xx response. The server reports owner-scoped resources as
`NotFound` (not `Forbidden`), so it does not confirm that other users' data
exists.

### Database migrations

The migrations are plain SQL in `migrations/`. `sqlx::migrate!` applies them at
boot. Add a new file with the next sequential prefix (`000N_description.sql`).
Never edit a migration that is already released. The in-memory adapter has no
schema, so mirror in the in-memory code any behavior that a migration implies
(for example, cascading deletes).

---

## 5. Coding standards

Each change must pass what CI checks:

```bash
cargo fmt --all --check                        # formatting
cargo clippy --all-targets -- -D warnings      # zero warnings
cargo test --all                               # tests green
```

- Match the style around your change (comment density, naming, and idioms).
- Prefer small, focused functions. Keep the HTTP handlers thin (parse → call the port → shape the output).
- Keep the in-memory and Postgres repositories identical in behavior.

---

## 6. Testing

- **Unit and integration tests** are next to the code in `#[cfg(test)]` modules.
  The in-memory adapters (`InMemoryEventStore`, `InMemoryUserRepository`) and
  small fakes (see the `application/event_service.rs` tests) let you test full
  use cases with no web server and no database.
- **Manual PDF checks**: render an invitation, then rasterize it to inspect it,
  for example `qlmanage -t -s 1600 -o out invite.pdf` (macOS) or `pdftoppm` (poppler).
- **Postgres path**: start the container from §2 and run your flow again, to
  confirm that the SQL adapters and the migrations behave correctly.

---

## 7. Commits and pull requests

- Branch off `master`. Keep each PR focused and reasonably small.
- Write clear commit messages in the imperative mood ("add event duplication").
- In the PR description, explain the **reason**, note each new env var or
  migration, and confirm that `fmt`, `clippy`, and `test` pass.
- Update the docs when behavior changes: the API table in `README.md`, the
  env-var table, and the Postman collection (`postman/rinvite.postman_collection.json`).

---

## 8. CI

GitHub Actions (`.github/workflows/ci.yml`) runs on each push and PR:

1. `cargo fmt --all --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --all`
4. `docker build` (confirms that the image still builds)

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

Thank you for your contribution.
