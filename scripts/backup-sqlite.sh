#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: $0 SOURCE_SQLITE DEST_SQLITE" >&2
    exit 2
fi

source_db=$1
destination_db=$2

if [ ! -f "$source_db" ]; then
    echo "SQLite source does not exist: $source_db" >&2
    exit 1
fi
command -v sqlite3 >/dev/null 2>&1 || command -v python3 >/dev/null 2>&1 || {
    echo "sqlite3 or python3 is required for an online backup" >&2
    exit 1
}

mkdir -p "$(dirname "$destination_db")"
tmp_destination="${destination_db}.tmp.$$"
cleanup() {
    rm -f "$tmp_destination"
}
trap cleanup EXIT INT TERM

# `.backup` uses SQLite's online backup API and is safe while the source is
# serving requests. Never fall back to cp/tar of the live database.
if command -v sqlite3 >/dev/null 2>&1; then
    sqlite3 "$source_db" ".timeout 5000" ".backup '$tmp_destination'"
else
    python3 -c 'import sqlite3,sys; source=sqlite3.connect("file:"+sys.argv[1]+"?mode=ro", uri=True); destination=sqlite3.connect(sys.argv[2]); source.backup(destination); destination.close(); source.close()' "$source_db" "$tmp_destination"
fi
test -s "$tmp_destination"
mv "$tmp_destination" "$destination_db"
trap - EXIT INT TERM
