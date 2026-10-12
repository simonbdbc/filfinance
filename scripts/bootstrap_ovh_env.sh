#!/usr/bin/env bash
set -euo pipefail

# Create the initial OVH production environment without ever printing a secret.
# The external credentials deliberately remain placeholders: the go-live gate
# blocks DNS activation until SMTP, FinancialJuice and OpenRouter are supplied.
ENV_FILE="${1:-.env.server.prod}"
TEMPLATE_FILE="${2:-.env.server.example}"
WORK_FILE=""

fail() {
  echo "OVH environment bootstrap failed: $*" >&2
  exit 1
}

generate_hex() {
  openssl rand -hex 32
}

env_value() {
  local name="$1"
  local file="$2"

  grep -E "^${name}=" "$file" | tail -n 1 | cut -d= -f2- | tr -d '\r'
}

replace_value() {
  local name="$1"
  local value="$2"
  local temporary

  temporary="$(mktemp "${WORK_FILE}.tmp.XXXXXX")"
  awk -v name="$name" -v value="$value" '
    index($0, name "=") == 1 {
      print name "=" value
      replaced = 1
      next
    }
    { print }
    END {
      if (!replaced) {
        exit 42
      }
    }
  ' "$WORK_FILE" > "$temporary" || {
    rm -f "$temporary"
    fail "missing required variable in template: $name"
  }
  mv "$temporary" "$WORK_FILE"
}

[ -r "$TEMPLATE_FILE" ] || fail "template is not readable: $TEMPLATE_FILE"
if [ -e "$ENV_FILE" ] || [ -L "$ENV_FILE" ]; then
  fail "refusing to overwrite existing environment file: $ENV_FILE"
fi
command -v openssl >/dev/null 2>&1 || fail 'openssl is required to generate secrets'

umask 077
WORK_FILE="$(mktemp "${ENV_FILE}.tmp.XXXXXX")"
cleanup() {
  [ -z "$WORK_FILE" ] || rm -f "$WORK_FILE"
}
trap cleanup EXIT
install -m 600 "$TEMPLATE_FILE" "$WORK_FILE"

postgres_password="$(generate_hex)"
auth_db_password="$(generate_hex)"
app_db_password="$(generate_hex)"
better_auth_secret="$(generate_hex)"
hls_ticket_secret="$(generate_hex)"
browser_ingest_token="$(generate_hex)"

auth_db_user="$(env_value FILFINANCE_AUTH_DB_USER "$WORK_FILE")"
app_db_user="$(env_value FILFINANCE_APP_DB_USER "$WORK_FILE")"
postgres_db="$(env_value POSTGRES_DB "$WORK_FILE")"

for value in "$auth_db_user" "$app_db_user" "$postgres_db"; do
  [ -n "$value" ] || fail 'database user or database name is empty in template'
done

replace_value POSTGRES_PASSWORD "$postgres_password"
replace_value FILFINANCE_AUTH_DB_PASSWORD "$auth_db_password"
replace_value FILFINANCE_APP_DB_PASSWORD "$app_db_password"
replace_value AUTH_DATABASE_URL "postgresql://${auth_db_user}:${auth_db_password}@filfinance-postgres:5432/${postgres_db}"
replace_value DATABASE_URL "postgresql://${app_db_user}:${app_db_password}@filfinance-postgres:5432/${postgres_db}"
replace_value BETTER_AUTH_SECRET "$better_auth_secret"
replace_value NEWS_HLS_ACCESS_TICKET_SECRET "$hls_ticket_secret"
replace_value NEWS_FINANCIAL_JUICE_BROWSER_INGEST_TOKEN "$browser_ingest_token"
chmod 600 "$WORK_FILE"
mv "$WORK_FILE" "$ENV_FILE"
WORK_FILE=""
trap - EXIT

echo "Created $ENV_FILE with generated internal secrets."
echo 'Set SMTP, FinancialJuice and OPENROUTER_API_KEY before the go-live preflight.'
