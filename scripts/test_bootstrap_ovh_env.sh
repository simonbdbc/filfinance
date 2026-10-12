#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BOOTSTRAP="$ROOT_DIR/scripts/bootstrap_ovh_env.sh"
TMP_DIR="$(mktemp -d)"
ENV_FILE="$TMP_DIR/.env.server.prod"
trap 'rm -rf "$TMP_DIR"' EXIT

assert_hex_secret() {
  local name="$1"
  local value

  value="$(grep -E "^${name}=" "$ENV_FILE" | cut -d= -f2-)"
  printf '%s' "$value" | grep -Eq '^[0-9a-f]{64}$' || {
    echo "$name is not a 32-byte hexadecimal secret" >&2
    exit 1
  }
}

file_mode() {
  local file="$1"

  if stat -c '%a' "$file" >/dev/null 2>&1; then
    stat -c '%a' "$file"
  else
    stat -f '%Lp' "$file"
  fi
}

output="$(bash "$BOOTSTRAP" "$ENV_FILE" "$ROOT_DIR/.env.server.example")"
test -f "$ENV_FILE"
test "$(file_mode "$ENV_FILE")" = 600

assert_hex_secret POSTGRES_PASSWORD
assert_hex_secret FILFINANCE_AUTH_DB_PASSWORD
assert_hex_secret FILFINANCE_APP_DB_PASSWORD
assert_hex_secret BETTER_AUTH_SECRET
assert_hex_secret NEWS_HLS_ACCESS_TICKET_SECRET
assert_hex_secret NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN

auth_password="$(grep '^FILFINANCE_AUTH_DB_PASSWORD=' "$ENV_FILE" | cut -d= -f2-)"
app_password="$(grep '^FILFINANCE_APP_DB_PASSWORD=' "$ENV_FILE" | cut -d= -f2-)"
grep -Fqx "AUTH_DATABASE_URL=postgresql://filfinance_auth:${auth_password}@filfinance-postgres:5432/filfinance" "$ENV_FILE"
grep -Fqx "DATABASE_URL=postgresql://filfinance_app:${app_password}@filfinance-postgres:5432/filfinance" "$ENV_FILE"
if printf '%s' "$output" | grep -Eq '[0-9a-f]{64}'; then
  echo 'bootstrap output must not contain a generated secret' >&2
  exit 1
fi
if bash "$BOOTSTRAP" "$ENV_FILE" "$ROOT_DIR/.env.server.example" >/dev/null 2>&1; then
  echo 'bootstrap must refuse to overwrite an existing environment file' >&2
  exit 1
fi

echo 'OVH environment bootstrap tests passed'
