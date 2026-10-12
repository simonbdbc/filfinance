#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATOR="$ROOT_DIR/scripts/migrate_server_env.sh"
TMP_DIR="$(mktemp -d)"
ENV_FILE="$TMP_DIR/.env.server.prod"
trap 'rm -rf "$TMP_DIR"' EXIT

printf '%s\n' \
  'NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=true' \
  'N8N_ENCRYPTION_KEY=obsolete-secret' \
  'N8N_POSTGRES_PASSWORD=obsolete-postgres-secret' \
  'FILFINANCE_N8N_WEBHOOK_SECRET=obsolete-webhook-secret' \
  'FILFINANCE_CALENDAR_NATIVE_SHADOW_ENABLED=true' \
  'FILFINANCE_CALENDAR_SHADOW_LOCAL_DAY=2026-08-28' \
  'FILFINANCE_CALENDAR_NATIVE_MAX_CONCURRENCY=1' \
  'OPENROUTER_API_KEY=active-native-secret' \
  'OPENROUTER_MODEL=nvidia/nemotron-3-super-120b-a12b:free' \
  > "$ENV_FILE"

output="$(bash "$MIGRATOR" "$ENV_FILE")"
printf '%s' "$output" | grep -Fq 'Migrated server env defaults; backup created at' || {
  echo "Expected legacy environment migration" >&2
  exit 1
}

! grep -Eq '^(N8N_|FILFINANCE_N8N_|FILFINANCE_CALENDAR_NATIVE_SHADOW_|FILFINANCE_CALENDAR_SHADOW_)' "$ENV_FILE" || {
  echo "Legacy n8n or shadow variables were not removed" >&2
  exit 1
}
grep -Fxq 'NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false' "$ENV_FILE"
grep -Fxq 'FILFINANCE_CALENDAR_NATIVE_MAX_CONCURRENCY=1' "$ENV_FILE"
grep -Fxq 'OPENROUTER_API_KEY=active-native-secret' "$ENV_FILE"
grep -Fxq 'OPENROUTER_MODEL=dots-studio/dots-3-note-preview:free' "$ENV_FILE"
grep -Fxq 'FILFINANCE_TRAEFIK_CERT_RESOLVER=lets_encrypt' "$ENV_FILE"
test -n "$(find "$TMP_DIR" -maxdepth 1 -name '.env.server.prod.bak-*' -print -quit)"

output="$(bash "$MIGRATOR" "$ENV_FILE")"
test "$output" = 'Server env defaults are already up to date'

for model in 'custom/model' '"nvidia/nemotron-3-super-120b-a12b:free"' "'nvidia/nemotron-3-super-120b-a12b:free'" 'minimax/minimax-m3:free' '"minimax/minimax-m3:free"' "'minimax/minimax-m3:free'"; do
  printf 'OPENROUTER_MODEL=%s\n' "$model" > "$TMP_DIR/quoted.prod"
  bash "$MIGRATOR" "$TMP_DIR/quoted.prod" >/dev/null
  case "$model" in
    custom/model) grep -Fxq 'OPENROUTER_MODEL=custom/model' "$TMP_DIR/quoted.prod" ;;
    *) grep -Fxq 'OPENROUTER_MODEL=dots-studio/dots-3-note-preview:free' "$TMP_DIR/quoted.prod" ;;
  esac
  test "$(bash "$MIGRATOR" "$TMP_DIR/quoted.prod")" = 'Server env defaults are already up to date'
done
printf 'OPENROUTER_MODEL=nvidia/nemotron-3-super-120b-a12b:free\n' > "$TMP_DIR/model.dev"
bash "$MIGRATOR" "$TMP_DIR/model.dev" >/dev/null
grep -Fxq 'OPENROUTER_MODEL=nvidia/nemotron-3-super-120b-a12b:free' "$TMP_DIR/model.dev"
printf 'OPENROUTER_MODEL=minimax/minimax-m3:free\n' > "$TMP_DIR/minimax.dev"
bash "$MIGRATOR" "$TMP_DIR/minimax.dev" >/dev/null
grep -Fxq 'OPENROUTER_MODEL=minimax/minimax-m3:free' "$TMP_DIR/minimax.dev"

echo "Server environment migration tests passed"
