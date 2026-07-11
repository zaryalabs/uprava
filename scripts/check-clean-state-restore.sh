#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
core_bin=${UPRAVA_REHEARSAL_CORE_BIN:-$repo/target/debug/uprava-server}
node_bin=${UPRAVA_REHEARSAL_NODE_BIN:-$repo/target/debug/uprava-node}
source_port=${UPRAVA_REHEARSAL_SOURCE_PORT:-28080}
restore_port=${UPRAVA_REHEARSAL_RESTORE_PORT:-28081}
tmp=$(mktemp -d)
pids=""
NO_PROXY="${NO_PROXY:+$NO_PROXY,}127.0.0.1,localhost"
no_proxy="${no_proxy:+$no_proxy,}127.0.0.1,localhost"
export NO_PROXY no_proxy

cleanup() {
    for pid in $pids; do kill "$pid" >/dev/null 2>&1 || true; done
    for pid in $pids; do wait "$pid" >/dev/null 2>&1 || true; done
    rm -rf "$tmp"
}
trap cleanup EXIT INT TERM

for command in curl python3 sqlite3; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "$command is required for clean-state rehearsal" >&2
        exit 1
    }
done
test -x "$core_bin"
test -x "$node_bin"

retained="$tmp/retained-0.1.8"
source_state="$tmp/core-0.2.0"
node_state="$tmp/node-0.2.0"
restore_state="$tmp/restored-core-0.2.0"
workspace="$tmp/workspace"
mkdir -p "$retained" "$source_state" "$node_state" "$restore_state" "$workspace"
printf '%s\n' retained-0.1.8 >"$retained/sentinel"
fake_codex="$tmp/codex"
printf '%s\n' '#!/bin/sh' 'if [ "${1:-}" = "--version" ]; then echo "codex-cli 0.0.0-rehearsal"; fi' >"$fake_codex"
chmod 755 "$fake_codex"

start_core() {
    port=$1
    state=$2
    UPRAVA_CORE_BIND="127.0.0.1:$port" \
    UPRAVA_DATABASE_URL="sqlite://$state/core.sqlite" \
    UPRAVA_CORE_LOG_FILE="$state/core.log" \
    UPRAVA_CLIENT_LOG_FILE="$state/client.log" \
    UPRAVA_ALLOWED_ORIGINS="http://127.0.0.1:$port" \
    UPRAVA_COOKIE_SECURE=false \
    "$core_bin" >"$state/stdout.log" 2>&1 &
    pid=$!
    pids="$pids $pid"
    attempt=0
    until curl -fsS "http://127.0.0.1:$port/api/v1/health" >/dev/null; do
        attempt=$((attempt + 1))
        test "$attempt" -lt 30 || { cat "$state/stdout.log" >&2; exit 1; }
        sleep 1
    done
}

json_field() {
    field=$1
    python3 -c 'import json,sys; value=json.load(sys.stdin)[sys.argv[1]]; print(value)' "$field"
}

start_core "$source_port" "$source_state"
source_url="http://127.0.0.1:$source_port/api/v1"
cookies="$tmp/cookies"
auth=$(curl -fsS -c "$cookies" -H "origin: http://127.0.0.1:$source_port" \
    -H 'content-type: application/json' \
    --data '{"password":"uprava-rehearsal-password"}' "$source_url/auth/setup")
csrf=$(printf '%s' "$auth" | json_field csrf_token)

UPRAVA_CORE_URL="http://127.0.0.1:$source_port" \
UPRAVA_NODE_DISPLAY_NAME="Clean Rehearsal Node" \
UPRAVA_NODE_HEARTBEAT_SECONDS=1 \
UPRAVA_NODE_STATE_PATH="$node_state/node.sqlite" \
UPRAVA_NODE_WORKSPACES="$workspace" \
UPRAVA_NODE_LOG_FILE="$node_state/node.log" \
UPRAVA_CODEX_BINARY="$fake_codex" \
"$node_bin" >"$node_state/stdout.log" 2>&1 &
node_pid=$!
pids="$pids $node_pid"

attempt=0
enrollment=""
until [ -n "$enrollment" ]; do
    enrollments=$(curl -fsS -b "$cookies" "$source_url/node-enrollments")
    enrollment=$(printf '%s' "$enrollments" | python3 -c '
import json,sys
for item in json.load(sys.stdin):
    if item.get("display_name") == "Clean Rehearsal Node":
        print(item["enrollment_id"])
        break
')
    attempt=$((attempt + 1))
    test "$attempt" -lt 30 || { cat "$node_state/stdout.log" >&2; exit 1; }
    [ -n "$enrollment" ] || sleep 1
done
curl -fsS -b "$cookies" -H "origin: http://127.0.0.1:$source_port" \
    -H "x-uprava-csrf: $csrf" -H 'content-type: application/json' \
    --data '{}' "$source_url/node-enrollments/$enrollment/approve" >/dev/null

attempt=0
until curl -fsS -b "$cookies" "$source_url/inventory" | grep -q 'Clean Rehearsal Node'; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 30 || { cat "$node_state/stdout.log" >&2; exit 1; }
    sleep 1
done
attempt=0
placement=""
until [ -n "$placement" ]; do
    inventory=$(curl -fsS -b "$cookies" "$source_url/inventory")
    placement=$(printf '%s' "$inventory" | python3 -c '
import json,sys
data=json.load(sys.stdin)
for item in data["placements"]:
    if item.get("workspace_path") == sys.argv[1]:
        print(item["project_placement_id"])
        break
' "$workspace")
    attempt=$((attempt + 1))
    test "$attempt" -lt 30 || exit 1
    [ -n "$placement" ] || sleep 1
done

session=$(curl -fsS -b "$cookies" -H "origin: http://127.0.0.1:$source_port" \
    -H "x-uprava-csrf: $csrf" \
    -H 'content-type: application/json' \
    --data "{\"project_placement_id\":\"$placement\",\"title\":\"Restore evidence session\",\"provider\":\"codex\"}" \
    "$source_url/sessions")
printf '%s' "$session" | grep -q 'Restore evidence session'

"$repo/scripts/backup-sqlite.sh" "$source_state/core.sqlite" "$tmp/online-backup.sqlite"
"$repo/scripts/verify-sqlite-backup.sh" "$tmp/online-backup.sqlite" >/dev/null
cp "$tmp/online-backup.sqlite" "$restore_state/core.sqlite"

start_core "$restore_port" "$restore_state"
restore_url="http://127.0.0.1:$restore_port/api/v1"
restored=$(curl -fsS -b "$cookies" "$restore_url/inventory")
printf '%s' "$restored" | grep -q 'Clean Rehearsal Node'
printf '%s' "$restored" | grep -q 'Restore evidence session'
test "$(cat "$retained/sentinel")" = retained-0.1.8
test -s "$node_state/node.sqlite"
test -s "$restore_state/core.sqlite"

echo "Clean Core/Node state-slot and separate online-backup restore rehearsal passed"
