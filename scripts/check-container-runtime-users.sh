#!/bin/sh
set -eu

check_image() {
    file=$1
    expected_user=$2

    final_stage=$(awk 'BEGIN { stage=0 } /^FROM[[:space:]]/ { stage++; last=stage } { lines[stage] = lines[stage] $0 "\n" } END { printf "%s", lines[last] }' "$file")
    user=$(printf '%s\n' "$final_stage" | awk '$1 == "USER" { value=$2 } END { print value }')

    if [ -z "$user" ] || [ "$user" = root ] || [ "$user" = 0 ] || [ "$user" = 0:0 ]; then
        echo "$file: final stage must declare a non-root USER" >&2
        exit 1
    fi
    if [ "$user" != "$expected_user" ]; then
        echo "$file: expected final USER $expected_user, got $user" >&2
        exit 1
    fi
    printf '%s: final USER %s\n' "$file" "$user"
}

check_image Dockerfile.core uprava
check_image Dockerfile.node uprava
check_image Dockerfile.toolhive 10002:10002
check_image Dockerfile.generated-ui-builder node
check_image apps/web/Dockerfile 101

for file in Dockerfile.core Dockerfile.node Dockerfile.toolhive Dockerfile.generated-ui-builder Dockerfile.codex-runtime apps/web/Dockerfile; do
    if grep '^FROM[[:space:]]' "$file" | grep -Ev '^FROM[[:space:]]+[^[:space:]]+@sha256:[0-9a-f]{64}([[:space:]]|$)' >/dev/null; then
        echo "$file: every release base must use an immutable digest" >&2
        exit 1
    fi
done

# OpenSandbox injects execd into this image as root, while Uprava supplies the
# workspace owner's uid/gid for every task command. Keep that exception explicit
# and verify the unprivileged execution identity is still present in the image.
if grep -q '^USER[[:space:]]' Dockerfile.codex-runtime; then
    echo "Dockerfile.codex-runtime: OpenSandbox runtime must retain its default root execd identity" >&2
    exit 1
fi
grep -q 'useradd --system --uid 10001 --gid task' Dockerfile.codex-runtime
grep -q 'npm install --global "@openai/codex@${CODEX_VERSION}"' Dockerfile.codex-runtime
grep -q '^ARG CODEX_VERSION=[0-9]' Dockerfile.codex-runtime

grep -q 'cargo build --locked --release -p uprava-server --bin uprava-server' Dockerfile.core
grep -q 'cargo build --locked --release -p uprava-node --bin uprava-node' Dockerfile.node
grep -q 'cargo build --locked --release -p uprava-toolhive --bin uprava-toolhive' Dockerfile.toolhive
grep -q 'TOOLHIVE_SOURCE_SHA256=2e3d5dd2a9be6a98ca72a6fdcc5e2f07e5a359bd48680352b5ba987fbcdec5fa' Dockerfile.toolhive
grep -q '0001-headless-encrypted-secrets.patch' Dockerfile.toolhive
grep -q '^RUN npm ci$' apps/web/Dockerfile
grep -q '^RUN npm ci$' Dockerfile.generated-ui-builder
grep -q '^CMD \["node", "services/generated-ui-builder/server.mjs"\]$' Dockerfile.generated-ui-builder
grep -q '^FROM nginxinc/nginx-unprivileged:' apps/web/Dockerfile
if awk '/^FROM / { stage++ } stage > 1 { print }' apps/web/Dockerfile | grep -Eq 'node_modules|npm run|vite'; then
    echo "apps/web/Dockerfile: runtime stage must be static-only" >&2
    exit 1
fi

grep -Eq '^ENV HOME=/var/lib/uprava([[:space:]]|\\$)' Dockerfile.core
grep -Eq '^ENV HOME=/var/lib/uprava([[:space:]]|\\$)' Dockerfile.node
grep -q '/data' Dockerfile.core
grep -q 'UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite' Dockerfile.node
grep -q '^USER 10002:10002$' Dockerfile.toolhive
grep -q 'os.Getenv(PasswordEnvVar)' patches/toolhive/0001-headless-encrypted-secrets.patch
grep -q 'ToolHive secrets password is missing or still uses the example value' ops/Makefile
grep -q 'UPRAVA_DOCKER_GID does not match /var/run/docker.sock' ops/Makefile
grep -q 'install -d -o 10001 -g 10001 -m 750 "$(INSTALL_DIR)/state/core"' Makefile
grep -q 'install -d -o 10002 -g 10002 -m 700 "$(INSTALL_DIR)/state/toolhive"' Makefile
grep -q 'wget -qO- http://127.0.0.1:8080/health' ci/finalize.sh
grep -q 'ca-certificates.crt /etc/ssl/certs/ca-certificates.crt' Dockerfile.core
grep -q 'ca-certificates.crt /etc/ssl/certs/ca-certificates.crt' Dockerfile.node
