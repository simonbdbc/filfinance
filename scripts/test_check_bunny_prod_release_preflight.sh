#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFLIGHT="$ROOT_DIR/scripts/check_bunny_prod_release_preflight.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

FAKE_BIN="$TMP_DIR/bin"
mkdir -p "$FAKE_BIN"

printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'url="${!#}"' \
  'case "$url" in' \
  '  */pullzone/101) echo "{\"Hostnames\":[\"filfinance.fr\"]}" ;;' \
  '  */pullzone/102)' \
  '    if [ "${FILFINANCE_TEST_BAD_PULL_ZONE:-}" = "102" ]; then' \
  '      echo "{\"Hostnames\":[\"wrong.example\"]}"' \
  '    else' \
  '      echo "{\"Hostnames\":[\"app.filfinance.fr\"]}"' \
  '    fi' \
  '    ;;' \
  '  *) exit 22 ;;' \
  'esac' \
  > "$FAKE_BIN/curl"
chmod +x "$FAKE_BIN/curl"

base_env=(
  env "PATH=$FAKE_BIN:$PATH"
  BUNNY_API_KEY_PROD=test-api-key
  BUNNY_STORAGE_ZONE_PROD_LANDING=filfinance-prod-landing
  BUNNY_STORAGE_PASSWORD_PROD_LANDING=test-landing-password
  BUNNY_PULL_ZONE_ID_PROD_LANDING=101
  BUNNY_STORAGE_ZONE_PROD_PWA=filfinance-prod-pwa
  BUNNY_STORAGE_PASSWORD_PROD_PWA=test-pwa-password
  BUNNY_PULL_ZONE_ID_PROD_PWA=102
  FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED=true
  FILFINANCE_PROD_PWA_URL=https://app.filfinance.fr
  BUNNY_API_ENDPOINT=https://bunny.test
)

output="$("${base_env[@]}" bash "$PREFLIGHT")"
printf '%s' "$output" | grep -Fq 'Bunny PROD preflight passed'
if printf '%s' "$output" | grep -Eq 'test-api-key|test-(landing|pwa)-password'; then
  echo "Preflight printed a credential" >&2
  exit 1
fi

if output="$(FILFINANCE_TEST_BAD_PULL_ZONE=102 "${base_env[@]}" bash "$PREFLIGHT" 2>&1)"; then
  echo "Expected hostname validation to fail" >&2
  exit 1
fi
printf '%s' "$output" | grep -Fq 'does not include hostname app.filfinance.fr'

echo "Bunny PROD preflight tests passed"
