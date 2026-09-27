#!/bin/sh
set -eu

umask 077

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

[ "$#" -eq 2 ] && [ "$2" = '--confirm-stopped' ] \
  || fail 'usage: scripts/docker-sqlite-restore.sh BACKUP --confirm-stopped'
command -v docker >/dev/null 2>&1 || fail 'docker is required'
command -v sqlite3 >/dev/null 2>&1 || fail 'host sqlite3 is required for integrity verification'
command -v curl >/dev/null 2>&1 || fail 'curl is required for readiness verification'

backup=$1
case "$backup" in
  /*) ;;
  *) backup="$PWD/$backup" ;;
esac
[ -f "$backup" ] || fail "backup does not exist: $backup"
result=$(sqlite3 "$backup" 'PRAGMA integrity_check;')
[ "$result" = 'ok' ] || fail "backup failed integrity check: $result"

compose_file=${COMPOSE_FILE:-deployment/docker-compose.yml}
[ -f "$compose_file" ] || fail "Compose file does not exist: $compose_file"
docker compose -f "$compose_file" ps -a --services | grep -qx server \
  || fail 'the Compose server container does not exist; run docker compose up once first'

docker compose -f "$compose_file" stop server
docker compose -f "$compose_file" run --rm --no-deps --entrypoint sh server -c \
  'for file in /data/indexlink.db-wal /data/indexlink.db-shm /data/indexlink.db-journal; do [ ! -e "$file" ] || { echo "SQLite sidecar remains: $file" >&2; exit 1; }; done'

mkdir -p "$PWD/backups"
timestamp=$(date -u '+%Y%m%dT%H%M%SZ')
pre_restore="$PWD/backups/pre-docker-restore-$timestamp.sqlite"
docker compose -f "$compose_file" cp server:/data/indexlink.db "$pre_restore"
result=$(sqlite3 "$pre_restore" 'PRAGMA integrity_check;')
[ "$result" = 'ok' ] || fail "current Docker database failed integrity check: $result"
printf 'Current Docker database backed up to: %s\n' "$pre_restore"

docker compose -f "$compose_file" cp "$backup" server:/data/indexlink.db.restore
docker compose -f "$compose_file" run --rm --no-deps --entrypoint sh server -c \
  'chmod 600 /data/indexlink.db.restore && mv /data/indexlink.db.restore /data/indexlink.db'

verification="$PWD/backups/.docker-restore-verification-$$.sqlite"
cleanup() {
  rm -f "$verification"
}
trap cleanup EXIT HUP INT TERM
docker compose -f "$compose_file" cp server:/data/indexlink.db "$verification"
result=$(sqlite3 "$verification" 'PRAGMA integrity_check;')
[ "$result" = 'ok' ] || fail "restored Docker database failed integrity check: $result"

docker compose -f "$compose_file" start server
ready=false
attempt=0
while [ "$attempt" -lt 30 ]; do
  if curl --fail --silent http://127.0.0.1:8080/ready >/dev/null 2>&1; then
    ready=true
    break
  fi
  attempt=$((attempt + 1))
  sleep 1
done
[ "$ready" = true ] || fail "server did not become ready; rollback backup is $pre_restore"

trap - EXIT HUP INT TERM
cleanup
printf 'Docker SQLite database restored from: %s\n' "$backup"
printf 'Integrity check: ok; /ready: ok\n'
