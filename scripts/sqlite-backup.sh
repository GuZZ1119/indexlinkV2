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
    ''|:memory:) fail 'backup requires a file-backed SQLite database' ;;
    *"'"*|*"
"*) fail 'database paths containing quotes or newlines are not supported' ;;
    /*) printf '%s\n' "$value" ;;
    *) printf '%s/%s\n' "$PWD" "$value" ;;
  esac
}

check_sqlite() {
  command -v sqlite3 >/dev/null 2>&1 || fail 'sqlite3 is required on the host'
}

check_integrity() {
  result=$(sqlite3 "$1" 'PRAGMA integrity_check;' 2>&1) || fail "SQLite integrity check failed for $1: $result"
  [ "$result" = 'ok' ] || fail "SQLite integrity check did not return ok for $1: $result"
}

write_checksum() {
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$(dirname "$1")" && sha256sum "$(basename "$1")") > "$1.sha256"
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$(dirname "$1")" && shasum -a 256 "$(basename "$1")") > "$1.sha256"
  fi
}

check_sqlite
database=$(resolve_database_path "${1:-${DATABASE_URL:-sqlite://indexlink.db?mode=rwc}}")
[ -f "$database" ] || fail "database does not exist: $database"

timestamp=$(date -u '+%Y%m%dT%H%M%SZ')
destination=${2:-"$PWD/backups/indexlink-$timestamp.sqlite"}
case "$destination" in
  *"'"*|*"
"*) fail 'backup paths containing quotes or newlines are not supported' ;;
  /*) ;;
  *) destination="$PWD/$destination" ;;
esac

[ "$database" != "$destination" ] || fail 'backup destination must differ from the live database'
[ ! -e "$destination" ] || fail "refusing to overwrite existing backup: $destination"
mkdir -p "$(dirname "$destination")"

temporary="$destination.tmp.$$"
cleanup() {
  rm -f "$temporary"
}
trap cleanup EXIT HUP INT TERM

# SQLite's online backup API produces a transactionally consistent snapshot,
# including when the application is using WAL mode. Never replace this with cp.
sqlite3 "$database" -cmd ".timeout 5000" ".backup '$temporary'" \
  || fail 'SQLite online backup failed'
check_integrity "$temporary"
mv "$temporary" "$destination"
write_checksum "$destination"
trap - EXIT HUP INT TERM

printf 'SQLite backup created: %s\n' "$destination"
printf 'Integrity check: ok\n'
[ ! -f "$destination.sha256" ] || printf 'Checksum: %s\n' "$destination.sha256"
