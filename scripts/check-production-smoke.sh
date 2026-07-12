#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

mkdir -p "$tmp/bin" "$tmp/state/core" "$tmp/state" "$tmp/node-state"
cat >"$tmp/.env.release" <<'EOF'
UPRAVA_RELEASE_ID=test-release
UPRAVA_RELEASE_SHA=test-release-sha
UPRAVA_NODE_VERSION=0.2.2
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server'
UPRAVA_STATE_EPOCH=0.2.2
UPRAVA_CORE_STATE_DIR=state/core
EOF
printf '%s\n' 0.2.2 >"$tmp/state/.state-epoch"
printf '%s\n' 0.2.2 >"$tmp/node-state/.state-epoch"

python3 - "$tmp/state/core/core.sqlite" <<'PY'
import datetime
import sqlite3
import sys

connection = sqlite3.connect(sys.argv[1])
connection.executescript(
    """
    create table nodes (
        node_id text primary key,
        display_name text not null,
        presence text not null,
        last_heartbeat_at text,
        daemon_version text not null,
        updated_at text not null
    );
    create table project_placements (
        project_placement_id text primary key,
        node_id text not null
    );
    create table core_schema_meta (
        slot text primary key,
        updated_at text not null
    );
    """
)
now = datetime.datetime.now(datetime.timezone.utc).isoformat()
connection.execute(
    "insert into nodes values (?, ?, ?, ?, ?, ?)",
    ("node-1", "Zarya Server", "reachable", now, "0.2.2", now),
)
connection.execute("insert into project_placements values (?, ?)", ("placement-1", "node-1"))
connection.execute("insert into core_schema_meta values (?, ?)", ("0.2.0", now))
connection.commit()
PY

cat >"$tmp/bin/curl" <<'SH'
#!/bin/sh
url=
for argument in "$@"; do url=$argument; done
case "$url" in
    */health) printf '%s\n' ok ;;
    */api/v1/version) printf '%s\n' '{"release_id":"test-release-sha"}' ;;
    *) exit 1 ;;
esac
SH
cat >"$tmp/bin/systemctl" <<'SH'
#!/bin/sh
test "$1" = is-active
SH
cat >"$tmp/bin/journalctl" <<'SH'
#!/bin/sh
exit 0
SH
chmod 755 "$tmp/bin/curl" "$tmp/bin/systemctl" "$tmp/bin/journalctl"

PATH="$tmp/bin:$PATH" \
INSTALL_DIR="$tmp" \
RELEASE_MANIFEST="$tmp/.env.release" \
NODE_STATE_EPOCH_MARKER="$tmp/node-state/.state-epoch" \
    "$repo/scripts/production-smoke.sh" >/dev/null

echo "Production smoke validates release, writable state, Node heartbeat and workspace projection"
