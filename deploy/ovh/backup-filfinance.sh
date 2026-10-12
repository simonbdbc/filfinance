#!/usr/bin/env bash
set -euo pipefail

# The timer runs this script as gitlab-ci, the owner of the deployment clone.
# It captures only durable application state. HLS clips, logs, models and the
# browser profile are intentionally excluded: they are either derived or can
# grow without bound on the 75 GB VPS.
APP_DIR="${FILFINANCE_APP_DIR:-/srv/filfinance}"
ENV_FILE="${FILFINANCE_ENV_FILE:-$APP_DIR/.env.server.prod}"
BACKUP_ROOT="${FILFINANCE_BACKUP_DIR:-$APP_DIR/backups}"
RETENTION_DAYS="${FILFINANCE_BACKUP_RETENTION_DAYS:-7}"
MIN_FREE_KB="${FILFINANCE_BACKUP_MIN_FREE_KB:-5242880}"
SQLITE_DB="$APP_DIR/volumes/service/data/news.db"

fail() {
  echo "FilFinance backup failed: $*" >&2
  exit 1
}

case "$APP_DIR" in
  /srv/filfinance) ;;
  *) fail "FILFINANCE_APP_DIR must remain /srv/filfinance" ;;
esac

case "$BACKUP_ROOT" in
  /srv/filfinance/backups) ;;
  *) fail "FILFINANCE_BACKUP_DIR must remain /srv/filfinance/backups" ;;
esac

case "$RETENTION_DAYS" in
  '' | *[!0-9]*) fail "FILFINANCE_BACKUP_RETENTION_DAYS must be a positive integer" ;;
esac
[ "$RETENTION_DAYS" -gt 0 ] || fail "FILFINANCE_BACKUP_RETENTION_DAYS must be positive"

for command in docker flock sqlite3 find df mktemp; do
  command -v "$command" >/dev/null 2>&1 || fail "missing required command: $command"
done

[ -r "$ENV_FILE" ] || fail "environment file is not readable: $ENV_FILE"
[ -f "$SQLITE_DB" ] || fail "SQLite database is missing: $SQLITE_DB"

umask 077
mkdir -p "$BACKUP_ROOT"
exec 9>"$BACKUP_ROOT/.backup.lock"
flock -n 9 || {
  echo "FilFinance backup already running; skipping" >&2
  exit 0
}

available_kb="$(df -Pk "$BACKUP_ROOT" | awk 'NR == 2 { print $4 }')"
case "$available_kb" in
  '' | *[!0-9]*) fail "unable to determine free disk space" ;;
esac
[ "$available_kb" -ge "$MIN_FREE_KB" ] || fail "less than ${MIN_FREE_KB} KB free for backup"

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
staging_dir="$(mktemp -d "$BACKUP_ROOT/.staging.XXXXXX")"
final_dir="$BACKUP_ROOT/$stamp"
cleanup() {
  rm -rf "$staging_dir"
}
trap cleanup EXIT

compose=(
  docker compose
  --project-directory "$APP_DIR"
  --env-file "$ENV_FILE"
  -f "$APP_DIR/compose.yaml"
  -f "$APP_DIR/compose.prod.yaml"
  -f "$APP_DIR/compose.ovh.yaml"
)

"${compose[@]}" exec -T filfinance-postgres sh -ceu \
  'pg_dump -U "$POSTGRES_USER" --format=custom "$POSTGRES_DB"' \
  > "$staging_dir/postgres.dump"
"${compose[@]}" exec -T filfinance-postgres pg_restore --list < "$staging_dir/postgres.dump" >/dev/null

# SQLite .backup takes a consistent snapshot while WAL is active.
sqlite3 "$SQLITE_DB" ".backup '$staging_dir/news.db'"
sqlite3 "$staging_dir/news.db" 'PRAGMA integrity_check;' | grep -Fxq 'ok' \
  || fail "SQLite integrity check failed"

cat > "$staging_dir/manifest.txt" <<EOF
created_at_utc=$stamp
postgres_dump=postgres.dump
sqlite_backup=news.db
retention_days=$RETENTION_DAYS
EOF

mv "$staging_dir" "$final_dir"
trap - EXIT

# Only completed timestamped backup directories are eligible for retention.
find "$BACKUP_ROOT" -mindepth 1 -maxdepth 1 -type d -name '20????????T??????Z' -mtime "+$RETENTION_DAYS" -exec rm -rf {} +
echo "FilFinance backup completed: $final_dir"
