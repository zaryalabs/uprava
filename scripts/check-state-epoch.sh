#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

mkdir -p "$tmp/core" "$tmp/node" "$tmp/markers"
for suffix in '' -wal -shm; do
    printf old >"$tmp/core/core.sqlite$suffix"
    printf old >"$tmp/node/node.sqlite$suffix"
done

"$repo/scripts/reset-state-epoch.sh" \
    0.2.2 \
    "$tmp/core/core.sqlite" \
    "$tmp/node/node.sqlite" \
    "$tmp/markers/core" \
    "$tmp/markers/node" >/dev/null

test "$(cat "$tmp/markers/core")" = 0.2.2
test "$(cat "$tmp/markers/node")" = 0.2.2
test ! -e "$tmp/core/core.sqlite"
test ! -e "$tmp/node/node.sqlite"

printf retained >"$tmp/core/core.sqlite"
"$repo/scripts/reset-state-epoch.sh" \
    0.2.2 \
    "$tmp/core/core.sqlite" \
    "$tmp/node/node.sqlite" \
    "$tmp/markers/core" \
    "$tmp/markers/node" >/dev/null
test "$(cat "$tmp/core/core.sqlite")" = retained

echo "State epoch reset is coordinated and idempotent"
