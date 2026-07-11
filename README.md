# auth-hexagonal

A minimal REST auth service (signup + login) structured with hexagonal
architecture. No ORM — plain SQL via sqlx. Swappable Postgres / in-memory
repository.

## Layout

```
src/
├── domain/                         # the hexagon: pure, no framework/db types
│   ├── model.rs                    # User entity
│   ├── error.rs                    # DomainError
│   └── port/                       # the two faces of the hexagon
│       ├── inbound.rs              # AuthService (driving port) + AuthToken
│       └── outbound.rs             # UserRepository / PasswordHasher / TokenIssuer
├── application/
│   └── auth_service.rs             # AuthServiceImpl: use-case logic, traits only
├── adapter/
│   ├── inbound/                    # driving adapters (call into the app)
│   │   └── http.rs                 # axum handlers, DTOs, error mapping
│   └── outbound/                   # driven adapters (the app calls out to)
│       ├── argon2_hasher.rs        # PasswordHasher impl
│       ├── jwt_issuer.rs           # TokenIssuer impl
│       └── persistence/
│           ├── memory.rs           # in-memory UserRepository
│           └── postgres.rs         # sqlx UserRepository
└── main.rs                         # composition root: wires adapters to ports
```

Dependency rule: everything points inward. `domain` depends on nothing;
`application` depends on `domain`; `adapter` and `main` depend on both.

## Run

`JWT_SECRET` is **required** (min 32 bytes) — the server refuses to start
without it, so it can never fall back to a guessable key. Generate one:

```bash
export JWT_SECRET=$(openssl rand -hex 32)
```

In-memory (no database needed):

```bash
cargo run
```

With Postgres:

```bash
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/auth
cargo run
```

### Docker

```bash
docker build -t rinvite .
docker run -p 3000:3000 -e JWT_SECRET=$(openssl rand -hex 32) rinvite
# add -e DATABASE_URL=... to use Postgres instead of the in-memory store
```

### Docker Compose (app + Postgres)

```bash
cp .env.example .env          # then set JWT_SECRET (openssl rand -hex 32)
docker compose up --build     # app on :3000, Postgres with a persistent volume
```

## Try it

```bash
# signup -> 201 with a JWT
curl -s -X POST localhost:3000/auth/signup \
  -H 'content-type: application/json' \
  -d '{"email":"a@b.com","password":"hunter2"}'

# login -> 200 with a JWT
curl -s -X POST localhost:3000/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"a@b.com","password":"hunter2"}'
```

## Adding another endpoint (the pattern)

1. Add the use-case method to the inbound port in `domain/port/inbound.rs`
   (e.g. add `me(&self, token: &str)` to `AuthService`, or create a new port
   trait like `ProfileService`).
2. If you need something new from the outside (a new query, a mailer, ...),
   add an outbound port trait in `domain/port/outbound.rs`.
3. Implement the use case in `application/` against those traits only.
4. Implement each new outbound port in `adapter/outbound/` (a Postgres impl, an
   in-memory impl, etc.).
5. Add a handler + route in `adapter/inbound/http.rs`.
6. Wire the new pieces in `main.rs`.

Steps 1–3 never mention axum or sqlx. That's the point.
