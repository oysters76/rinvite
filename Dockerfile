# ===== Build stage =========================================================
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# Copy only what the build needs (see .dockerignore for what's excluded).
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

# ===== Runtime stage =======================================================
# sqlx uses rustls (no OpenSSL) and argon2 is pure Rust, so the only runtime
# dependency is ca-certificates for TLS to Postgres.
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rinvite /usr/local/bin/rinvite

EXPOSE 3000
# Run as a non-root user.
USER 1000:1000

# JWT_SECRET (>=32 bytes) is required at runtime; the process refuses to boot
# without it. Pass it with `-e JWT_SECRET=...` (and DATABASE_URL for Postgres).
ENTRYPOINT ["rinvite"]
