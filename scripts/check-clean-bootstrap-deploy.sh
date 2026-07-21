#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
mkdir -p "$tmp/bin" "$tmp/releases/test" "$tmp/state/core" "$tmp/state/toolhive"
printf 'UPRAVA_DOMAIN=example.test\n' >"$tmp/core.env"
printf 'UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite\n' >"$tmp/node.env"
printf 'UPRAVA_DOCKER_GID=990\nTOOLHIVE_SECRETS_PASSWORD=test-only\n' >"$tmp/toolhive.env"
chmod 600 "$tmp/toolhive.env"
printf '#!/bin/sh\nexit 0\n' >"$tmp/node-binary"
node_sha=$(sha256sum "$tmp/node-binary" | awk '{print $1}')
cat >"$tmp/releases/test.env.release" <<EOF
UPRAVA_RELEASE_ID=test
UPRAVA_RELEASE_SHA=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
UPRAVA_RELEASE_FAMILY=0.2.0
UPRAVA_CORE_STATE_DIR=state/core
UPRAVA_TOOLHIVE_STATE_DIR=state/toolhive
UPRAVA_CORE_CONFIG=$tmp/core.env
UPRAVA_NODE_CONFIG=$tmp/node.env
UPRAVA_TOOLHIVE_CONFIG=$tmp/toolhive.env
UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server'
UPRAVA_TOOLHIVE_PROFILE=toolhive
UPRAVA_TOOLHIVE_VERSION=0.40.0
UPRAVA_CORE_IMAGE=ghcr.io/zaryalabs/uprava-core@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
UPRAVA_WEB_IMAGE=ghcr.io/zaryalabs/uprava-web@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
UPRAVA_GENERATED_UI_BUILDER_IMAGE=ghcr.io/zaryalabs/uprava-generated-ui-builder@sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd
UPRAVA_NODE_ARTIFACT=ghcr.io/zaryalabs/uprava-node@sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
UPRAVA_TOOLHIVE_IMAGE=ghcr.io/zaryalabs/uprava-toolhive@sha256:1111111111111111111111111111111111111111111111111111111111111111
UPRAVA_TASK_RUNTIME_IMAGE=ghcr.io/zaryalabs/uprava-codex-runtime@sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee
UPRAVA_OPENSANDBOX_IMAGE=opensandbox/server@sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
UPRAVA_NODE_SHA256=$node_sha
UPRAVA_NODE_VERSION=0.2.3
EOF

cat >"$tmp/bin/docker" <<SH
#!/bin/sh
printf '%s\n' "\$*" >>"$tmp/docker.log"
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
    TOOLHIVE_CONFIG="$tmp/toolhive.env" COMPOSE_FILE="$repo/ops/compose.yaml" SUDO= >/dev/null

test "$(readlink "$tmp/.env.release")" = "$tmp/releases/test.env.release"
test "$(readlink "$tmp/current")" = "$tmp/releases/test"
test -x "$tmp/releases/test/uprava-node"
test -f "$tmp/backups/pre-deploy/test/no-core-state"
grep -Eq -- 'compose .*--profile toolhive .* up -d --remove-orphans' "$tmp/docker.log"
echo "Clean bootstrap deploy rehearsal passed in an isolated fixture"
