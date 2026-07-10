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

if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "sqlite3 is required for backup/restore rehearsal" >&2
    exit 1
fi

sqlite3 "$source_db" <<'SQL'
PRAGMA foreign_keys = ON;
CREATE TABLE rehearsal_inventory (id INTEGER PRIMARY KEY, value TEXT NOT NULL);
INSERT INTO rehearsal_inventory(id, value) VALUES (1, 'deterministic-backup-fixture');
SQL

mkdir -p "$backup_dir"
"$repo_dir/scripts/backup-sqlite.sh" "$source_db" "$backup_dir/core.sqlite"
tar -czf "$archive" -C "$backup_dir" core.sqlite

# Exercise the same archive verification path used by operators.
(cd "$repo_dir/ops" && make --no-print-directory restore-verify BACKUP="$archive" >/dev/null)

mkdir -p "$tmp_dir/restored"
tar -xzf "$archive" -C "$tmp_dir/restored"
value=$(sqlite3 "$tmp_dir/restored/core.sqlite" \
    "SELECT value FROM rehearsal_inventory WHERE id = 1;")
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
