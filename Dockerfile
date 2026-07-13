# ===== Build stage =========================================================
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# Copy only what the build needs (see .dockerignore for what's excluded).
# `assets/` is required at *compile* time: html.rs and message/mod.rs embed the
# e-invite and message templates via `include_str!` relative to CARGO_MANIFEST_DIR.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY assets ./assets

RUN cargo build --release

# ===== Runtime stage =======================================================
# sqlx uses rustls (no OpenSSL) and argon2 is pure Rust, so the only runtime
# dependency is ca-certificates for TLS to Postgres.
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/rinvite /usr/local/bin/rinvite
# The branded print PDF reads the card image + TTF fonts at *runtime* from the
# relative paths in assets/pdf-config.json. Ship them and point PDF_CONFIG at
# the config so the PDF renders the floral-gold card instead of a plain fallback.
# CWD /app makes the config's relative asset paths resolve; the files are
# world-readable, so USER 1000 can read them.
COPY --from=builder /app/assets ./assets
ENV PDF_CONFIG=/app/assets/pdf-config.json

EXPOSE 3000
# Run as a non-root user.
USER 1000:1000

# JWT_SECRET (>=32 bytes) is required at runtime; the process refuses to boot
# without it. Pass it with `-e JWT_SECRET=...` (and DATABASE_URL for Postgres).
ENTRYPOINT ["rinvite"]
