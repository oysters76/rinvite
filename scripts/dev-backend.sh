#!/usr/bin/env bash
# Dev backend launcher for the Claude Preview MCP (supplies required env).
cd "$(dirname "$0")/.."
export JWT_SECRET="dev-secret-please-change-0123456789abcdef"
export PDF_CONFIG="assets/pdf-config.json"
export PUBLIC_BASE_URL="http://localhost:3000"
exec target/debug/rinvite
