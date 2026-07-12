#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
releases="$tmp/builds/releases"
mkdir -p "$releases"

for release in active newest middle oldest; do
    mkdir -p "$releases/$release"
    printf 'UPRAVA_RELEASE_ID=%s\n' "$release" >"$releases/$release.env.release"
    sleep 1
done
ln -s "$releases/active.env.release" "$tmp/.env.release"

INSTALL_DIR="$tmp" UPRAVA_KEEP_RELEASES=2 \
    "$repo/scripts/prune-uprava-releases.sh" >/dev/null

test -f "$releases/active.env.release"
test -f "$releases/oldest.env.release"
test -f "$releases/middle.env.release"
test ! -e "$releases/newest.env.release"

echo "Release retention preserves the active release and the newest bounded set"
