#!/bin/sh
set -eu

umask 077

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

command -v docker >/dev/null 2>&1 || fail 'docker is required'
command -v sqlite3 >/dev/null 2>&1 || fail 'host sqlite3 is required for integrity verification'
compose_file=${COMPOSE_FILE:-deployment/docker-compose.yml}
[ -f "$compose_file" ] || fail "Compose file does not exist: $compose_file"

timestamp=$(date -u '+%Y%m%dT%H%M%SZ')
destination=${1:-"$PWD/backups/indexlink-docker-$timestamp.sqlite"}
case "$destination" in
  /*) ;;
  *) destination="$PWD/$destination" ;;
esac
[ ! -e "$destination" ] || fail "refusing to overwrite existing backup: $destination"
mkdir -p "$(dirname "$destination")"

was_running=false
if docker compose -f "$compose_file" ps --status running --services | grep -qx server; then
  was_running=true
fi
restart_if_needed() {
  if [ "$was_running" = true ]; then
    docker compose -f "$compose_file" start server >/dev/null || true
  fi
}
trap restart_if_needed EXIT HUP INT TERM

docker compose -f "$compose_file" stop server
docker compose -f "$compose_file" run --rm --no-deps --entrypoint sh server -c \
  'for file in /data/indexlink.db-wal /data/indexlink.db-shm /data/indexlink.db-journal; do [ ! -e "$file" ] || { echo "SQLite sidecar remains: $file" >&2; exit 1; }; done'

temporary="$destination.tmp.$$"
docker compose -f "$compose_file" cp server:/data/indexlink.db "$temporary"
result=$(sqlite3 "$temporary" 'PRAGMA integrity_check;')
[ "$result" = 'ok' ] || fail "copied database failed integrity check: $result"
mv "$temporary" "$destination"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$(dirname "$destination")" && sha256sum "$(basename "$destination")") > "$destination.sha256"
elif command -v shasum >/dev/null 2>&1; then
  (cd "$(dirname "$destination")" && shasum -a 256 "$(basename "$destination")") > "$destination.sha256"
fi

trap - EXIT HUP INT TERM
restart_if_needed
printf 'Docker SQLite backup created: %s\n' "$destination"
printf 'Integrity check: ok\n'
