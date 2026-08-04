#!/usr/bin/env bash
# Dev backend launcher for the Claude Preview MCP (supplies required env).
cd "$(dirname "$0")/.."
export JWT_SECRET="dev-secret-please-change-0123456789abcdef"
export PDF_CONFIG="assets/pdf-config.json"
# Re-read PDF_CONFIG (and its image/fonts) on every render so edits to
# assets/pdf-config.json show up on a plain repeated request — no restart.
export PDF_CONFIG_RELOAD=1
# Local Postgres (the `rinvite-pg` dev container). Enables login/approval and
# persists accounts across restarts. Unset this to fall back to in-memory repos.
export DATABASE_URL="${DATABASE_URL:-postgres://auth:auth@localhost:5432/auth}"
export PUBLIC_BASE_URL="http://localhost:3000"
# Invite links fall back to PUBLIC_BASE_URL. Set this to test the reverse-proxy
# host that fronts /i/* in production:
#   export INVITE_BASE_URL="https://rinvite.ceykod.com"
exec target/debug/rinvite
