#!/bin/sh
set -eu

umask 077

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

resolve_database_path() {
  value=$1
  case "$value" in
    sqlite://*) value=${value#sqlite://} ;;
    sqlite:*) value=${value#sqlite:} ;;
  esac
  value=${value%%\?*}
  value=${value%%\#*}
  case "$value" in
    ''|:memory:) fail 'restore requires a file-backed SQLite database' ;;
    *"'"*|*"
"*) fail 'database paths containing quotes or newlines are not supported' ;;
    /*) printf '%s\n' "$value" ;;
    *) printf '%s/%s\n' "$PWD" "$value" ;;
  esac
}

check_integrity() {
  result=$(sqlite3 "$1" 'PRAGMA integrity_check;' 2>&1) || fail "SQLite integrity check failed for $1: $result"
  [ "$result" = 'ok' ] || fail "SQLite integrity check did not return ok for $1: $result"
}

verify_checksum() {
  [ ! -f "$1.sha256" ] && return 0
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$(dirname "$1")" && sha256sum -c "$(basename "$1").sha256") >/dev/null
  elif command -v shasum >/dev/null 2>&1; then
    expected=$(sed -n '1{s/[[:space:]].*$//;p;}' "$1.sha256")
    actual=$(shasum -a 256 "$1" | sed 's/[[:space:]].*$//')
    [ "$expected" = "$actual" ] || fail 'backup checksum mismatch'
  else
    fail 'a checksum sidecar exists but sha256sum/shasum is unavailable'
  fi
}

[ "$#" -ge 1 ] || fail 'usage: scripts/sqlite-restore.sh BACKUP [DATABASE_URL_OR_PATH] --confirm-stopped'
backup=$1
database_value=${DATABASE_URL:-sqlite://indexlink.db?mode=rwc}
confirmed=false
shift
for argument in "$@"; do
  case "$argument" in
    --confirm-stopped) confirmed=true ;;
    *) database_value=$argument ;;
  esac
done
[ "$confirmed" = true ] || fail 'stop IndexLink first, then pass --confirm-stopped'
command -v sqlite3 >/dev/null 2>&1 || fail 'sqlite3 is required on the host'
command -v lsof >/dev/null 2>&1 || fail 'lsof is required to prove that the database is not open'
script_directory=$(CDPATH= cd -- "$(dirname "$0")" && pwd)

case "$backup" in
  *"'"*|*"
"*) fail 'backup paths containing quotes or newlines are not supported' ;;
  /*) ;;
  *) backup="$PWD/$backup" ;;
esac
database=$(resolve_database_path "$database_value")
[ -f "$backup" ] || fail "backup does not exist: $backup"
[ "$backup" != "$database" ] || fail 'backup and restore target must differ'

verify_checksum "$backup"
check_integrity "$backup"

if lsof "$database" "$database-wal" "$database-shm" "$database-journal" >/dev/null 2>&1; then
  fail 'database is still open; stop IndexLink before restoring'
fi
for sidecar in "$database-wal" "$database-shm" "$database-journal"; do
  [ ! -e "$sidecar" ] || fail "refusing restore while SQLite sidecar exists: $sidecar"
done

directory=$(dirname "$database")
mkdir -p "$directory" "$PWD/backups"
timestamp=$(date -u '+%Y%m%dT%H%M%SZ')
pre_restore="$PWD/backups/pre-restore-$timestamp.sqlite"
if [ -f "$database" ]; then
  "$script_directory/sqlite-backup.sh" "$database" "$pre_restore" >/dev/null
  printf 'Current database backed up to: %s\n' "$pre_restore"
fi

stage="$directory/.$(basename "$database").restore.$$"
cleanup() {
  rm -f "$stage"
}
trap cleanup EXIT HUP INT TERM
sqlite3 "$stage" ".restore '$backup'" || fail 'SQLite restore into staging database failed'
check_integrity "$stage"
chmod 600 "$stage"
mv -f "$stage" "$database"
check_integrity "$database"
trap - EXIT HUP INT TERM

printf 'SQLite database restored: %s\n' "$database"
printf 'Integrity check: ok\n'
printf 'Restart IndexLink and verify: curl --fail http://127.0.0.1:8080/ready\n'
