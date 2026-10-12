#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-${FILFINANCE_ENV_FILE:-.env.server}}"
TARGET="${2:-prod}"
DEV_PWA_URL="${3:-${FILFINANCE_DEV_PWA_URL:-}}"

fail() {
  echo "Better Auth environment preflight failed: $*" >&2
  exit 1
}

env_value() {
  local name="$1"
  local value

  value="$(grep -E "^${name}=" "$ENV_FILE" | tail -n 1 | cut -d= -f2- | tr -d '\r' || true)"
  value="${value#\"}"
  value="${value%\"}"
  value="${value#\'}"
  value="${value%\'}"
  printf '%s' "$value"
}

require_value() {
  local name="$1"
  local normalized
  local value

  value="$(env_value "$name")"
  [ -n "$value" ] || fail "$name is missing or empty in $ENV_FILE"
  normalized="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  case "$normalized" in
    *replace-with* | *change-me* | *example.com* | *example.net* | *example.test*)
      fail "$name still contains a placeholder in $ENV_FILE"
      ;;
  esac
}

require_equal() {
  local name="$1"
  local expected="$2"
  local actual

  actual="$(env_value "$name")"
  [ "$actual" = "$expected" ] || fail "$name must equal $expected"
}

require_boolean() {
  local name="$1"
  local value

  value="$(env_value "$name")"
  case "$value" in
    true | false) ;;
    *) fail "$name must equal true or false" ;;
  esac
}

require_integer_range() {
  local name="$1"
  local minimum="$2"
  local maximum="$3"
  local value

  value="$(env_value "$name")"
  case "$value" in
    '' | *[!0-9]*) fail "$name must be an integer between $minimum and $maximum" ;;
  esac
  [ "$value" -ge "$minimum" ] && [ "$value" -le "$maximum" ] \
    || fail "$name must be an integer between $minimum and $maximum"
}

require_https_origin() {
  local name="$1"
  local value="$2"
  local authority
  local normalized_authority

  case "$value" in
    https://*) ;;
    *) fail "$name must be an HTTPS origin" ;;
  esac
  authority="${value#https://}"
  [ -n "$authority" ] || fail "$name must include a host"
  case "$authority" in
    */* | *\?* | *\#* | *,* | *" "*) fail "$name must be an origin without path, query, fragment, comma, or spaces" ;;
  esac
  normalized_authority="$(printf '%s' "$authority" | tr '[:upper:]' '[:lower:]')"
  case "$normalized_authority" in
    *replace-with* | example.* | *.example.* | *.invalid | *.invalid:*) fail "$name must not contain a placeholder host" ;;
  esac
}

require_trusted_origin() {
  local origin="$1"

  printf '%s' "$trusted_origins" \
    | tr ',' '\n' \
    | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' \
    | grep -Fqx -- "$origin" \
    || fail "$origin must be listed in BETTER_AUTH_TRUSTED_ORIGINS"
}

trusted_origin_present() {
  local origin="$1"

  printf '%s' "$trusted_origins" \
    | tr ',' '\n' \
    | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' \
    | grep -Fqx -- "$origin"
}

forbid_trusted_origin() {
  local origin="$1"

  if trusted_origin_present "$origin"; then
    fail "$origin must not be listed in BETTER_AUTH_TRUSTED_ORIGINS while the landing does not call Better Auth"
  fi
}

forbid_cross_environment_origin() {
  local origin="$1"

  if trusted_origin_present "$origin"; then
    fail "$origin belongs to the other environment and must not be listed in BETTER_AUTH_TRUSTED_ORIGINS"
  fi
}

[ -r "$ENV_FILE" ] || fail "$ENV_FILE is not readable by $(id -un)"
case "$TARGET" in
  dev | prod) ;;
  *) fail "target must be dev or prod" ;;
esac

for name in \
  FILFINANCE_API_HOST \
  POSTGRES_DB POSTGRES_USER POSTGRES_PASSWORD \
  FILFINANCE_AUTH_DB_USER FILFINANCE_AUTH_DB_PASSWORD \
  FILFINANCE_APP_DB_USER FILFINANCE_APP_DB_PASSWORD \
  AUTH_DATABASE_URL DATABASE_URL \
  BETTER_AUTH_URL BETTER_AUTH_SECRET BETTER_AUTH_JWT_AUDIENCE BETTER_AUTH_JWT_EXPIRATION \
  BETTER_AUTH_TRUSTED_ORIGINS APP_WEB_URL \
  EXPO_SCHEME \
  NEWS_AUTH_ENABLED NEWS_AUTH_PROVIDER NEWS_AUTH_ISSUER NEWS_AUTH_JWKS_URL NEWS_AUTH_AUDIENCE \
  NEWS_AUTH_MODERATOR_ROLES
do
  require_value "$name"
done

if [ "$TARGET" = "prod" ]; then
  expected_api_host="api.filfinance.fr"
else
  expected_api_host="api.dev.filfinance.fr"
fi
require_equal FILFINANCE_API_HOST "$expected_api_host"

api_url="https://$expected_api_host"
require_equal BETTER_AUTH_URL "$api_url"
require_equal BETTER_AUTH_JWT_AUDIENCE "$api_url"
require_equal BETTER_AUTH_JWT_EXPIRATION "15m"
require_equal NEWS_AUTH_ENABLED "true"
require_equal NEWS_AUTH_PROVIDER "better_auth"
require_equal NEWS_AUTH_ISSUER "$api_url"
require_equal NEWS_AUTH_AUDIENCE "$api_url"
require_equal NEWS_AUTH_JWKS_URL "http://filfinance-auth:3000/api/auth/jwks"

moderator_roles="$(env_value NEWS_AUTH_MODERATOR_ROLES)"
normalized_moderator_roles="$(printf '%s' "$moderator_roles" | tr -d '[:space:]')"
case "$normalized_moderator_roles" in
  moderator,admin | admin,moderator) ;;
  *) fail "NEWS_AUTH_MODERATOR_ROLES must contain exactly moderator,admin" ;;
esac

secret="$(env_value BETTER_AUTH_SECRET)"
[ "${#secret}" -ge 32 ] || fail "BETTER_AUTH_SECRET must contain at least 32 characters"
postgres_user="$(env_value POSTGRES_USER)"
auth_db_user="$(env_value FILFINANCE_AUTH_DB_USER)"
app_db_user="$(env_value FILFINANCE_APP_DB_USER)"
[ "$postgres_user" != "$auth_db_user" ] \
  && [ "$postgres_user" != "$app_db_user" ] \
  && [ "$auth_db_user" != "$app_db_user" ] \
  || fail "POSTGRES_USER, FILFINANCE_AUTH_DB_USER and FILFINANCE_APP_DB_USER must be distinct"
case "$(env_value AUTH_DATABASE_URL)" in
  postgresql://"$auth_db_user":* | postgres://"$auth_db_user":*) ;;
  *) fail "AUTH_DATABASE_URL username must match FILFINANCE_AUTH_DB_USER" ;;
esac
case "$(env_value DATABASE_URL)" in
  postgresql://"$app_db_user":* | postgres://"$app_db_user":*) ;;
  *) fail "DATABASE_URL username must match FILFINANCE_APP_DB_USER" ;;
esac

admin_email="$(env_value ADMIN_EMAIL_TO_PROMOTE)"
if [ -n "$admin_email" ]; then
  require_value ADMIN_EMAIL_TO_PROMOTE
  case "$admin_email" in
    *@*.*) ;;
    *) fail "ADMIN_EMAIL_TO_PROMOTE must be empty or a valid email address" ;;
  esac
fi

if [ "$TARGET" = "dev" ]; then
  require_https_origin FILFINANCE_DEV_PWA_URL "$DEV_PWA_URL"
  [ "$DEV_PWA_URL" = "https://app.dev.filfinance.fr" ] \
    || fail "FILFINANCE_DEV_PWA_URL must equal https://app.dev.filfinance.fr"
fi

trusted_origins="$(env_value BETTER_AUTH_TRUSTED_ORIGINS)"
if [ "$TARGET" = "prod" ]; then
  expected_app_web_url="https://app.filfinance.fr"
  expected_pwa_origin="https://app.filfinance.fr"
  expected_landing_origin="https://filfinance.fr"
  forbidden_landing_origin="https://dev.filfinance.fr"
else
  expected_app_web_url="$DEV_PWA_URL"
  expected_pwa_origin="$DEV_PWA_URL"
  forbidden_pwa_origin="https://app.filfinance.fr"
  expected_landing_origin="https://dev.filfinance.fr"
  forbidden_landing_origin="https://filfinance.fr"
fi
require_equal APP_WEB_URL "$expected_app_web_url"
require_equal EXPO_SCHEME "filfinance"
require_trusted_origin "filfinance://"
require_trusted_origin "$expected_pwa_origin"
if [ "$TARGET" = "dev" ]; then
  forbid_cross_environment_origin "$forbidden_pwa_origin"
fi

require_equal BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION "true"
require_value SMTP_HOST
require_value SMTP_USER
require_value SMTP_PASSWORD
require_value SMTP_FROM_EMAIL
require_value SMTP_FROM_NAME
require_integer_range SMTP_PORT 1 65535
require_boolean SMTP_SECURE
require_boolean SMTP_REQUIRE_TLS
require_equal SMTP_HOST smtp-relay.brevo.com
require_equal SMTP_PORT 587
require_equal SMTP_SECURE false
require_equal SMTP_REQUIRE_TLS true
require_equal SMTP_FROM_EMAIL noreply@filfinance.fr

landing_uses_auth="$(env_value FILFINANCE_LANDING_USES_BETTER_AUTH)"
case "$landing_uses_auth" in
  '' | false)
    forbid_trusted_origin "$expected_landing_origin"
    forbid_cross_environment_origin "$forbidden_landing_origin"
    ;;
  true)
    require_trusted_origin "$expected_landing_origin"
    forbid_cross_environment_origin "$forbidden_landing_origin"
    ;;
  *) fail "FILFINANCE_LANDING_USES_BETTER_AUTH must equal true or false" ;;
esac

while IFS= read -r origin; do
  [ -n "$origin" ] || continue
  if [ "$origin" = "filfinance://" ] \
    || [ "$origin" = "$expected_pwa_origin" ]; then
    continue
  fi
  if [ "$landing_uses_auth" = "true" ] \
    && [ "$origin" = "$expected_landing_origin" ]; then
    continue
  fi
  if [ "$TARGET" = "dev" ] \
    && { [ "$origin" = "exp://" ] || [ "$origin" = "exp://**" ] || [ "$origin" = "exp://192.168.*.*:*/**" ]; }; then
    continue
  fi
  fail "$origin is not an allowed deployed Better Auth origin"
done < <(
  printf '%s\n' "$trusted_origins" \
    | tr ',' '\n' \
    | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'
)

if [ "$TARGET" = "prod" ]; then
  case "$trusted_origins" in
    *exp://*) fail "Expo development origins are forbidden in production" ;;
  esac
fi

echo "Better Auth environment preflight passed for $TARGET"
