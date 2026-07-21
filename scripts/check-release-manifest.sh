#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
mkdir -p "$tmp/bin" "$tmp/builds/releases/test"

cat >"$tmp/bin/docker" <<'SH'
#!/bin/sh
case "$1:$2" in
    buildx:imagetools)
        printf '%s\n' '"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"'
        ;;
    image:inspect)
        exit 0
        ;;
    create:*)
        printf '%s\n' fake-container
        ;;
    cp:*)
        printf '%s\n' '#!/bin/sh' 'exit 0' >"$3"
        ;;
    rm:*)
        exit 0
        ;;
    *)
        echo "Unexpected docker command: $*" >&2
        exit 1
        ;;
esac
SH
chmod 755 "$tmp/bin/docker"

cd "$repo"
PATH="$tmp/bin:$PATH" \
RELEASE_MANIFEST="$tmp/builds/releases/test.env.release" \
RELEASE_ID=test \
GIT_SHA=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb \
BUILD_TIMESTAMP=2026-07-12T00:00:00Z \
UPRAVA_CORE_IMAGE=ghcr.io/zaryalabs/uprava-core:sha-test \
UPRAVA_WEB_IMAGE=ghcr.io/zaryalabs/uprava-web:sha-test \
UPRAVA_GENERATED_UI_BUILDER_IMAGE=ghcr.io/zaryalabs/uprava-generated-ui-builder:sha-test \
UPRAVA_NODE_IMAGE=ghcr.io/zaryalabs/uprava-node:sha-test \
UPRAVA_TASK_RUNTIME_IMAGE=ghcr.io/zaryalabs/uprava-codex-runtime:sha-test \
UPRAVA_OPENSANDBOX_IMAGE=opensandbox/server:v0.2.2 \
UPRAVA_NODE_VERSION=0.2.2 \
UPRAVA_RELEASE_FAMILY=0.2.0 \
UPRAVA_CORE_STATE_DIR=state/core \
UPRAVA_CORE_CONFIG=/etc/uprava/core.env \
UPRAVA_NODE_CONFIG=/etc/uprava/node.env \
UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite \
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server' \
NODE_ARTIFACT_PATH="$tmp/builds/releases/test/uprava-node" \
    scripts/write_release_manifest.sh >/dev/null

# shellcheck disable=SC1090
. "$tmp/builds/releases/test.env.release"
test "$UPRAVA_AUTO_APPROVE_NODE_NAME" = 'Zarya Server'
case "$UPRAVA_CORE_IMAGE:$UPRAVA_WEB_IMAGE:$UPRAVA_GENERATED_UI_BUILDER_IMAGE:$UPRAVA_NODE_ARTIFACT:$UPRAVA_TASK_RUNTIME_IMAGE:$UPRAVA_OPENSANDBOX_IMAGE" in
    *@sha256:*@sha256:*@sha256:*@sha256:*@sha256:*@sha256:*) ;;
    *) echo "Release images are not digest-pinned" >&2; exit 1 ;;
esac
test -n "$UPRAVA_NODE_SHA256"

test "$UPRAVA_CORE_CONFIG" = /etc/uprava/core.env
grep -Fq 'UPRAVA_NODE_VERSION ?= $(UPRAVA_NODE_PACKAGE_VERSION)+$(GIT_SHA)' "$repo/Makefile"
echo "Release manifest is shell-safe, state-neutral and digest-pins every runtime artifact"
