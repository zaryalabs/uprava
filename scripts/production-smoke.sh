#!/bin/sh
set -eu

install_dir=${INSTALL_DIR:-/opt/apps/uprava}
manifest=${RELEASE_MANIFEST:-$install_dir/.env.release}
retries=${SMOKE_RETRIES:-60}
delay_seconds=${SMOKE_DELAY_SECONDS:-1}

test -f "$manifest" || {
    echo "Missing release manifest: $manifest" >&2
    exit 1
}

# shellcheck disable=SC1090
. "$manifest"

: "${UPRAVA_RELEASE_SHA:?manifest is missing UPRAVA_RELEASE_SHA}"
: "${UPRAVA_NODE_VERSION:?manifest is missing UPRAVA_NODE_VERSION}"
: "${UPRAVA_AUTO_APPROVE_NODE_NAME:?manifest is missing UPRAVA_AUTO_APPROVE_NODE_NAME}"
: "${UPRAVA_STATE_EPOCH:?manifest is missing UPRAVA_STATE_EPOCH}"
: "${UPRAVA_CORE_STATE_DIR:?manifest is missing UPRAVA_CORE_STATE_DIR}"

domain=${UPRAVA_DOMAIN:-uprava.zrya.io}
core_database="$install_dir/$UPRAVA_CORE_STATE_DIR/core.sqlite"
node_epoch_marker=${NODE_STATE_EPOCH_MARKER:-/var/lib/uprava-node/.state-epoch}

command -v curl >/dev/null 2>&1 || {
    echo "curl is required for production smoke" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "python3 is required for production smoke" >&2
    exit 1
}

curl -fsS "https://$domain/health" | grep -qx ok
version=$(curl -fsS "https://$domain/api/v1/version")
printf '%s' "$version" | python3 -c '
import json, sys
expected = sys.argv[1]
actual = json.load(sys.stdin).get("release_id")
if actual != expected:
    raise SystemExit(f"Core release mismatch: expected {expected}, got {actual}")
' "$UPRAVA_RELEASE_SHA"

systemctl is-active --quiet uprava-node.service

attempt=1
while [ "$attempt" -le "$retries" ]; do
    if python3 - "$core_database" "$UPRAVA_AUTO_APPROVE_NODE_NAME" "$UPRAVA_NODE_VERSION" <<'PY'
import datetime
import sqlite3
import sys

database, expected_name, expected_version = sys.argv[1:]
connection = sqlite3.connect(f"file:{database}?mode=rw", uri=True)
if connection.execute("pragma quick_check").fetchone() != ("ok",):
    raise SystemExit(1)
connection.execute("begin immediate")
connection.execute("update core_schema_meta set updated_at = updated_at where 0")
connection.rollback()
row = connection.execute(
    """
    select display_name, presence, last_heartbeat_at, daemon_version, node_id
    from nodes
    where display_name = ?
    order by updated_at desc
    limit 1
    """,
    (expected_name,),
).fetchone()
if row is None:
    raise SystemExit(1)
display_name, presence, last_heartbeat_at, daemon_version, node_id = row
if presence != "reachable" or daemon_version != expected_version or not last_heartbeat_at:
    raise SystemExit(1)
heartbeat = datetime.datetime.fromisoformat(last_heartbeat_at.replace("Z", "+00:00"))
now = datetime.datetime.now(datetime.timezone.utc)
if heartbeat.tzinfo is None:
    heartbeat = heartbeat.replace(tzinfo=datetime.timezone.utc)
if (now - heartbeat).total_seconds() > 45:
    raise SystemExit(1)
workspace_count = connection.execute(
    "select count(*) from project_placements where node_id = ?",
    (node_id,),
).fetchone()[0]
if workspace_count < 1:
    raise SystemExit(1)
print(f"Production Node ready: {display_name}, version {daemon_version}, workspaces {workspace_count}")
PY
    then
        test "$(sed -n '1p' "$install_dir/state/.state-epoch")" = "$UPRAVA_STATE_EPOCH"
        test "$(sed -n '1p' "$node_epoch_marker")" = "$UPRAVA_STATE_EPOCH"
        echo "Production smoke passed for $UPRAVA_RELEASE_SHA"
        exit 0
    fi
    sleep "$delay_seconds"
    attempt=$((attempt + 1))
done

echo "Production Node did not become ready" >&2
journalctl -u uprava-node.service -n 80 --no-pager >&2 || true
exit 1
