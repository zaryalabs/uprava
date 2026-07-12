#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

cat >"$tmp/Makefile" <<'MAKE'
activate:
	@test "$(RELEASE)" = candidate
	@test "$(DOCKER_CONFIG)" = /tmp/uprava-docker-config
	@printf '%s\n' activate >> calls

deploy:
	@test "$(DOCKER_CONFIG)" = /tmp/uprava-docker-config
	@printf '%s\n' deploy >> calls
MAKE

RELEASE_ID=candidate \
DEPLOY_MODE=local \
INSTALL_DIR="$tmp" \
DOCKER_CONFIG=/tmp/uprava-docker-config \
SUDO= \
    "$repo/scripts/deploy.sh"

test "$(sed -n '1p' "$tmp/calls")" = activate
test "$(sed -n '2p' "$tmp/calls")" = deploy
echo "Deploy entrypoint activates once and preserves the temporary Docker config"
