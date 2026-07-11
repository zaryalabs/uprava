#!/bin/sh
set -eu

# Local release-gate rehearsal. It only uses a temporary SQLite database and
# never reads or writes the product's configured state directory.
repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

source_db="$tmp_dir/source.sqlite"
backup_dir="$tmp_dir/backup"
archive="$tmp_dir/uprava-core-data.tgz"
corrupt_archive="$tmp_dir/uprava-core-data-corrupt.tgz"

python3 -c 'import sqlite3,sys; connection=sqlite3.connect(sys.argv[1]); connection.executescript("PRAGMA foreign_keys = ON; CREATE TABLE rehearsal_inventory (id INTEGER PRIMARY KEY, value TEXT NOT NULL); INSERT INTO rehearsal_inventory(id, value) VALUES (1, '\''deterministic-backup-fixture'\'');"); connection.commit(); connection.close()' "$source_db"

mkdir -p "$backup_dir"
"$repo_dir/scripts/backup-sqlite.sh" "$source_db" "$backup_dir/core.sqlite"
tar -czf "$archive" -C "$backup_dir" core.sqlite

# Exercise the same archive verification path used by operators.
(cd "$repo_dir/ops" && make --no-print-directory restore-verify BACKUP="$archive" >/dev/null)

mkdir -p "$tmp_dir/restored"
tar -xzf "$archive" -C "$tmp_dir/restored"
value=$(python3 -c 'import sqlite3,sys; connection=sqlite3.connect(sys.argv[1]); print(connection.execute("SELECT value FROM rehearsal_inventory WHERE id = 1").fetchone()[0]); connection.close()' "$tmp_dir/restored/core.sqlite")
test "$value" = deterministic-backup-fixture || {
    echo "backup/restore rehearsal lost fixture data" >&2
    exit 1
}

# A non-archive must be rejected by the restore verification path. This is
# intentionally separate so the valid rehearsal archive remains intact.
printf '%s\n' 'corrupt backup fixture' >"$corrupt_archive"
if (cd "$repo_dir/ops" && make --no-print-directory restore-verify \
    BACKUP="$corrupt_archive" >/dev/null 2>&1); then
    echo "backup/restore rehearsal accepted a corrupt archive" >&2
    exit 1
fi

echo "Backup/restore rehearsal passed (temporary SQLite fixture only)"
