#!/usr/bin/env sh
set -eu

: "${POSTGRES_DB:?Missing POSTGRES_DB}"
: "${FILFINANCE_AUTH_DB_USER:?Missing FILFINANCE_AUTH_DB_USER}"
: "${FILFINANCE_AUTH_DB_PASSWORD:?Missing FILFINANCE_AUTH_DB_PASSWORD}"
: "${FILFINANCE_APP_DB_USER:?Missing FILFINANCE_APP_DB_USER}"
: "${FILFINANCE_APP_DB_PASSWORD:?Missing FILFINANCE_APP_DB_PASSWORD}"

psql -v ON_ERROR_STOP=1 \
  --username "$POSTGRES_USER" \
  --dbname "$POSTGRES_DB" \
  -v auth_user="$FILFINANCE_AUTH_DB_USER" \
  -v auth_password="$FILFINANCE_AUTH_DB_PASSWORD" \
  -v app_user="$FILFINANCE_APP_DB_USER" \
  -v app_password="$FILFINANCE_APP_DB_PASSWORD" <<'EOSQL'
CREATE EXTENSION IF NOT EXISTS pgcrypto;

SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', :'auth_user', :'auth_password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'auth_user') \gexec

SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', :'app_user', :'app_password')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = :'app_user') \gexec

CREATE SCHEMA IF NOT EXISTS auth;
CREATE SCHEMA IF NOT EXISTS app;

ALTER SCHEMA auth OWNER TO :"auth_user";
ALTER SCHEMA app OWNER TO :"app_user";

GRANT USAGE, CREATE ON SCHEMA auth TO :"auth_user";
GRANT USAGE, CREATE ON SCHEMA app TO :"app_user";

ALTER ROLE :"auth_user" SET search_path TO auth;
ALTER ROLE :"app_user" SET search_path TO app, public;
EOSQL
