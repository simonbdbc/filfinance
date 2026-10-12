#!/usr/bin/env bash
# Verifie les prerequis techniques de publication des deux sites statiques PROD.
# Les reponses Bunny et les secrets ne sont jamais ecrits dans les logs.
set -euo pipefail

fail() { echo "Bunny PROD preflight: $*" >&2; exit 1; }
require() { [ -n "${!1:-}" ] || fail "Missing protected GitLab variable $1"; }
expect() { [ "${!1:-}" = "$2" ] || fail "$1 must equal $2"; }

for variable in \
  BUNNY_API_KEY_PROD \
  BUNNY_STORAGE_ZONE_PROD_LANDING BUNNY_STORAGE_PASSWORD_PROD_LANDING BUNNY_PULL_ZONE_ID_PROD_LANDING \
  BUNNY_STORAGE_ZONE_PROD_PWA BUNNY_STORAGE_PASSWORD_PROD_PWA BUNNY_PULL_ZONE_ID_PROD_PWA \
  FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED FILFINANCE_PROD_PWA_URL; do
  require "$variable"
done

expect FILFINANCE_PROD_BUNNY_DEPLOY_ENABLED true
expect FILFINANCE_PROD_PWA_URL https://app.filfinance.fr
expect BUNNY_STORAGE_ZONE_PROD_LANDING filfinance-prod-landing
expect BUNNY_STORAGE_ZONE_PROD_PWA filfinance-prod-pwa
[ "$BUNNY_PULL_ZONE_ID_PROD_LANDING" != "$BUNNY_PULL_ZONE_ID_PROD_PWA" ] || \
  fail "production landing and PWA Pull Zones must be distinct"

api_base="${BUNNY_API_ENDPOINT:-https://api.bunny.net}"
api_base="${api_base%/}"
get_pull_zone() {
  curl --fail --silent --show-error --retry 2 \
    -H "AccessKey: ${BUNNY_API_KEY_PROD}" \
    "$api_base/pullzone/$1"
}
contains() { jq -e --arg value "$2" 'tostring | contains($value)' >/dev/null <<<"$1"; }

check_pull_zone() {
  local id="$1" hostname="$2" body
  body="$(get_pull_zone "$id")" || fail "unable to read Pull Zone $id"
  contains "$body" "$hostname" || fail "Pull Zone $id does not include hostname $hostname"
}

# Le certificat Bunny est controle apres le CNAME : ce preflight doit aussi
# pouvoir valider une publication initiale sur le hostname *.b-cdn.net.
check_pull_zone "$BUNNY_PULL_ZONE_ID_PROD_LANDING" filfinance.fr
check_pull_zone "$BUNNY_PULL_ZONE_ID_PROD_PWA" app.filfinance.fr

echo "Bunny PROD preflight passed; verify Bunny SSL/Edge Rules before the DNS cutover."
