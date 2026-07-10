#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 SQLITE_BACKUP" >&2
    exit 2
fi

database=$1
if [ ! -f "$database" ]; then
    echo "SQLite backup does not exist: $database" >&2
    exit 1
fi
if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "sqlite3 is required for backup verification" >&2
    exit 1
fi

integrity="$(sqlite3 "$database" 'pragma integrity_check;')"
test "$integrity" = ok || {
    echo "SQLite integrity_check failed: $integrity" >&2
    exit 1
}

foreign_keys="$(sqlite3 "$database" 'pragma foreign_key_check;')"
test -z "$foreign_keys" || {
    echo "SQLite foreign_key_check failed: $foreign_keys" >&2
    exit 1
}

echo "SQLite backup verified: $database"
