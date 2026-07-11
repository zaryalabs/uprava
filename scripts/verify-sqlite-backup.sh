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
if command -v sqlite3 >/dev/null 2>&1; then
    integrity="$(sqlite3 "$database" 'pragma integrity_check;')"
    foreign_keys="$(sqlite3 "$database" 'pragma foreign_key_check;')"
elif command -v python3 >/dev/null 2>&1; then
    integrity="$(python3 -c 'import sqlite3,sys; connection=sqlite3.connect(sys.argv[1]); print(connection.execute("pragma integrity_check").fetchone()[0]); connection.close()' "$database")"
    foreign_keys="$(python3 -c 'import sqlite3,sys; connection=sqlite3.connect(sys.argv[1]); rows=connection.execute("pragma foreign_key_check").fetchall(); print("" if not rows else rows); connection.close()' "$database")"
else
    echo "sqlite3 or python3 is required for backup verification" >&2
    exit 1
fi
test "$integrity" = ok || {
    echo "SQLite integrity_check failed: $integrity" >&2
    exit 1
}

test -z "$foreign_keys" || {
    echo "SQLite foreign_key_check failed: $foreign_keys" >&2
    exit 1
}

echo "SQLite backup verified: $database"
