#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd "$script_dir/.." && pwd)"
cd "$project_dir"

: "${AUTH_DATABASE_URL:?AUTH_DATABASE_URL must point to the Better Auth test database}"
: "${DATABASE_URL:?DATABASE_URL must point to the application test database}"

export PORT="${PORT:-3000}"
export BETTER_AUTH_URL="${BETTER_AUTH_URL:-http://localhost:${PORT}}"
export BETTER_AUTH_SECRET="${BETTER_AUTH_SECRET:-filfinance-local-integration-secret-at-least-32-characters}"
export BETTER_AUTH_JWT_AUDIENCE="${BETTER_AUTH_JWT_AUDIENCE:-$BETTER_AUTH_URL}"
export BETTER_AUTH_TRUSTED_ORIGINS="${BETTER_AUTH_TRUSTED_ORIGINS:-$BETTER_AUTH_URL,http://localhost:8052}"
export BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION="true"
export APP_WEB_URL="${APP_WEB_URL:-http://localhost:8052}"
export SMTP_HOST="127.0.0.1"
export FILFINANCE_SMTP_CAPTURE_PORT="${FILFINANCE_SMTP_CAPTURE_PORT:-2525}"
export SMTP_PORT="$FILFINANCE_SMTP_CAPTURE_PORT"
export SMTP_SECURE="false"
export SMTP_REQUIRE_TLS="false"
export SMTP_FROM_EMAIL="${SMTP_FROM_EMAIL:-noreply@filfinance.test}"
export SMTP_FROM_NAME="${SMTP_FROM_NAME:-FilFinance Integration}"
export FILFINANCE_AUTH_BASE_URL="${FILFINANCE_AUTH_BASE_URL:-http://127.0.0.1:${PORT}}"
export FILFINANCE_AUTH_SMOKE_FROM_EMAIL="$SMTP_FROM_EMAIL"
export FILFINANCE_AUTH_SMOKE_FROM_NAME="$SMTP_FROM_NAME"

integration_tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/filfinance-auth-integration.XXXXXX")"
chmod 700 "$integration_tmp_dir"
export FILFINANCE_SMTP_CAPTURE_FILE="$integration_tmp_dir/verification.eml"
export FILFINANCE_SMTP_READY_FILE="$integration_tmp_dir/smtp.ready"
export FILFINANCE_AUTH_SMOKE_EMAIL_FILE="$FILFINANCE_SMTP_CAPTURE_FILE"

smtp_pid=""
auth_pid=""
cleanup() {
  [ -z "$auth_pid" ] || kill "$auth_pid" 2>/dev/null || true
  [ -z "$smtp_pid" ] || kill "$smtp_pid" 2>/dev/null || true
  [ -z "$auth_pid" ] || wait "$auth_pid" 2>/dev/null || true
  [ -z "$smtp_pid" ] || wait "$smtp_pid" 2>/dev/null || true
  rm -rf "$integration_tmp_dir"
}
trap cleanup EXIT INT TERM

npm run build
node scripts/smtp_capture_server.mjs &
smtp_pid=$!
node -e 'const fs = require("fs"); const sleep = ms => new Promise(resolve => setTimeout(resolve, ms)); (async () => { for (let attempt = 0; attempt < 30; attempt += 1) { if (fs.existsSync(process.env.FILFINANCE_SMTP_READY_FILE)) return; await sleep(200); } throw new Error("SMTP capture did not become ready"); })()'

node dist/index.js &
auth_pid=$!
node -e 'const sleep = ms => new Promise(resolve => setTimeout(resolve, ms)); (async () => { for (let attempt = 0; attempt < 30; attempt += 1) { try { const response = await fetch(`${process.env.FILFINANCE_AUTH_BASE_URL}/ready`); if (response.ok) return; } catch {} await sleep(1000); } throw new Error("Better Auth did not become ready"); })()'

npm run smoke:integration
