#!/usr/bin/env bash
# Vérifie les prérequis contrôlables avant une publication Bunny DEV. Les politiques Edge Rules
# restent aussi consignées dans le runbook pour revue humaine: aucun secret n'est affiché ici.
set -euo pipefail

fail() { echo "Bunny DEV preflight: $*" >&2; exit 1; }
require() { [ -n "${!1:-}" ] || fail "Missing protected GitLab variable $1"; }
expect() { [ "${!1:-}" = "$2" ] || fail "$1 must equal $2"; }

for variable in \
  BUNNY_API_KEY_DEV BUNNY_DNS_ZONE_ID_DEV \
  BUNNY_STORAGE_ZONE_DEV_LANDING BUNNY_STORAGE_PASSWORD_DEV_LANDING BUNNY_PULL_ZONE_ID_DEV_LANDING \
  BUNNY_STORAGE_ZONE_DEV_PWA BUNNY_STORAGE_PASSWORD_DEV_PWA BUNNY_PULL_ZONE_ID_DEV_PWA \
  BUNNY_PULL_ZONE_ID_DEV_API BUNNY_PULL_ZONE_ID_DEV_MEDIA BUNNY_PULL_ZONE_ID_DEV_LIVE; do
  require "$variable"
done

expect FILFINANCE_DEV_LANDING_URL https://dev.filfinance.fr
expect FILFINANCE_DEV_PWA_URL https://app.dev.filfinance.fr
expect FILFINANCE_DEV_API_URL https://api.dev.filfinance.fr
expect FILFINANCE_DEV_HLS_BASE_URL https://media.dev.filfinance.fr
expect FILFINANCE_DEV_EDGE_URL https://live.dev.filfinance.fr
expect FILFINANCE_DEV_ORIGIN_URL https://origin.dev.filfinance.fr

api_base="${BUNNY_API_ENDPOINT:-https://api.bunny.net}"
api_base="${api_base%/}"
get_resource() {
  curl --fail --silent --show-error --retry 2 \
    -H "AccessKey: ${BUNNY_API_KEY_DEV}" "$api_base/$1/$2"
}
get_collection() {
  curl --fail --silent --show-error --retry 2 \
    -H "AccessKey: ${BUNNY_API_KEY_DEV}" "$api_base/$1"
}
contains() { jq -e --arg value "$2" 'tostring | contains($value)' >/dev/null <<<"$1"; }

dns="$(get_resource dnszone "$BUNNY_DNS_ZONE_ID_DEV")" || fail "unable to read Bunny DEV DNS zone"
contains "$dns" "dev.filfinance.fr" || fail "Bunny DNS zone does not describe dev.filfinance.fr"

check_pull_zone() {
  local id="$1" host="$2" origin="${3:-}" body
  body="$(get_resource pullzone "$id")" || fail "unable to read Pull Zone $id"
  contains "$body" "$host" || fail "Pull Zone $id does not include hostname $host"
  if [ -n "$origin" ]; then
    contains "$body" "$origin" || fail "Pull Zone $id does not use origin $origin"
  fi
}

check_pull_zone "$BUNNY_PULL_ZONE_ID_DEV_LANDING" dev.filfinance.fr
check_pull_zone "$BUNNY_PULL_ZONE_ID_DEV_PWA" app.dev.filfinance.fr
check_pull_zone "$BUNNY_PULL_ZONE_ID_DEV_API" api.dev.filfinance.fr origin.dev.filfinance.fr
check_pull_zone "$BUNNY_PULL_ZONE_ID_DEV_MEDIA" media.dev.filfinance.fr origin.dev.filfinance.fr
check_pull_zone "$BUNNY_PULL_ZONE_ID_DEV_LIVE" live.dev.filfinance.fr origin.dev.filfinance.fr

# A Shield Advanced plan has wider rule quotas. Basic is identified here by its
# documented two rate-limit slots and zero custom WAF slots; this makes the
# 5 USD DEV budget an executable gate instead of a dashboard-only review.
shield_zones="$(get_collection shield/shield-zones)" || fail "unable to read Bunny Shield zones"
check_shield_basic() {
  local pull_zone_id="$1" label="$2" shield
  shield="$(jq -ce --arg id "$pull_zone_id" '
    .data[] | select((.pullZoneId | tostring) == $id)
  ' <<<"$shield_zones" || true)"
  [ -n "$shield" ] || fail "Shield zone is missing for $label Pull Zone $pull_zone_id"
  jq -e '
    .rateLimitRulesLimit == 2 and .customWafRulesLimit == 0
  ' >/dev/null <<<"$shield" || fail "Shield zone for $label is not Bunny Shield Basic"

  printf '%s' "$shield"
}

api_shield="$(check_shield_basic "$BUNNY_PULL_ZONE_ID_DEV_API" API)"
media_shield="$(check_shield_basic "$BUNNY_PULL_ZONE_ID_DEV_MEDIA" HLS)"
live_shield="$(check_shield_basic "$BUNNY_PULL_ZONE_ID_DEV_LIVE" live)"

# There are exactly two route-specific limits in this pilot: Better Auth on
# the API Pull Zone and the live WebSocket handshake. Query Bunny's rate-limit
# collection rather than trusting a plan quota alone.
check_rate_limit_route() {
  local shield="$1" route="$2" label="$3" shield_zone_id rate_limits
  shield_zone_id="$(jq -er '.shieldZoneId' <<<"$shield")" || fail "missing Shield zone id for $label"
  rate_limits="$(get_collection "shield/rate-limits/$shield_zone_id?perPage=10")" \
    || fail "unable to read Shield rate limits for $label"
  jq -e --arg route "$route" '
    [(.data // [])[] | ((.ruleJson // "") + " " + (.ruleName // "") + " " + ((.ruleConfiguration // {}) | tostring))]
    | any(contains($route))
  ' >/dev/null <<<"$rate_limits" || fail "Shield rate limit for $label must target $route"
}

check_rate_limit_route "$api_shield" /api/auth/ "API Better Auth"
check_rate_limit_route "$live_shield" /ws/live "live WebSocket"

live_pull_zone="$(get_resource pullzone "$BUNNY_PULL_ZONE_ID_DEV_LIVE")" || fail "unable to read live Pull Zone"
jq -e '.. | objects | select(has("EnableWebSockets")) | .EnableWebSockets == true' >/dev/null <<<"$live_pull_zone" \
  || fail "live Pull Zone must enable WebSockets"

media_pull_zone="$(get_resource pullzone "$BUNNY_PULL_ZONE_ID_DEV_MEDIA")" || fail "unable to read HLS Pull Zone"
contains "$media_pull_zone" "POST" || fail "HLS Pull Zone must retain its POST-blocking Edge Rule"

echo "Bunny DEV preflight passed; verify HLS CORS and origin Bunny-only firewall from docs/BUNNY-DEV-RUNBOOK.md."
