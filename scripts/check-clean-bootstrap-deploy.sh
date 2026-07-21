#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
mkdir -p "$tmp/bin" "$tmp/releases/test" "$tmp/state/core"
printf 'UPRAVA_DOMAIN=example.test\n' >"$tmp/core.env"
printf 'UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite\n' >"$tmp/node.env"
printf '#!/bin/sh\nexit 0\n' >"$tmp/node-binary"
node_sha=$(sha256sum "$tmp/node-binary" | awk '{print $1}')
cat >"$tmp/releases/test.env.release" <<EOF
UPRAVA_RELEASE_ID=test
UPRAVA_RELEASE_SHA=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
UPRAVA_RELEASE_FAMILY=0.2.0
UPRAVA_CORE_STATE_DIR=state/core
UPRAVA_CORE_CONFIG=$tmp/core.env
UPRAVA_NODE_CONFIG=$tmp/node.env
UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server'
UPRAVA_CORE_IMAGE=ghcr.io/zaryalabs/uprava-core@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
UPRAVA_WEB_IMAGE=ghcr.io/zaryalabs/uprava-web@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
UPRAVA_GENERATED_UI_BUILDER_IMAGE=ghcr.io/zaryalabs/uprava-generated-ui-builder@sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd
UPRAVA_NODE_ARTIFACT=ghcr.io/zaryalabs/uprava-node@sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
UPRAVA_NODE_SHA256=$node_sha
UPRAVA_NODE_VERSION=0.2.3
EOF

cat >"$tmp/bin/docker" <<SH
#!/bin/sh
if [ "\$1" = --config ]; then shift 2; fi
case "\$1:\$2" in
    compose:*) exit 0 ;;
    pull:*) exit 0 ;;
    create:*) printf '%s\n' fake-container ;;
    cp:*) cp "$tmp/node-binary" "\$3" ;;
    rm:*) exit 0 ;;
    *) echo "unexpected docker command: \$*" >&2; exit 1 ;;
esac
SH
cat >"$tmp/bin/systemctl" <<'SH'
#!/bin/sh
exit 0
SH
chmod 755 "$tmp/bin/docker" "$tmp/bin/systemctl"

cd "$tmp"
PATH="$tmp/bin:$PATH" make -f "$repo/ops/Makefile" --no-print-directory deploy \
    RELEASE=test RELEASES_DIR="$tmp/releases" CORE_CONFIG="$tmp/core.env" NODE_CONFIG="$tmp/node.env" \
    COMPOSE_FILE="$repo/ops/compose.yaml" SUDO= >/dev/null

test "$(readlink "$tmp/.env.release")" = "$tmp/releases/test.env.release"
test "$(readlink "$tmp/current")" = "$tmp/releases/test"
test -x "$tmp/releases/test/uprava-node"
echo "Clean bootstrap deploy rehearsal passed in an isolated fixture"
