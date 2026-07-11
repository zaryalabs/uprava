#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
mkdir -p "$tmp/builds/releases/prior" "$tmp/builds/releases/candidate"
printf '%s\n' UPRAVA_RELEASE_ID=prior UPRAVA_CORE_CONFIG=prior-core.env UPRAVA_NODE_CONFIG=prior-node.env UPRAVA_CORE_STATE_DIR=state/prior >"$tmp/builds/releases/prior.env.release"
printf '%s\n' UPRAVA_RELEASE_ID=candidate UPRAVA_CORE_CONFIG=candidate-core.env UPRAVA_NODE_CONFIG=candidate-node.env UPRAVA_CORE_STATE_DIR=state/candidate >"$tmp/builds/releases/candidate.env.release"
ln -s "$tmp/builds/releases/prior.env.release" "$tmp/.env.release"
ln -s "$tmp/builds/releases/prior" "$tmp/current"
ln -s prior-core.env "$tmp/.env"
ln -s prior-node.env "$tmp/node.env"
ln -s state/prior "$tmp/core-state"

cat >"$tmp/Makefile" <<'MAKE'
SHELL := /bin/sh
RELEASES := builds/releases

activate:
	@ln -sfn "$(CURDIR)/$(RELEASES)/$(RELEASE).env.release" .env.release
	@ln -sfn "$(CURDIR)/$(RELEASES)/$(RELEASE)" current
	@. ./.env.release; ln -sfn "$$UPRAVA_CORE_CONFIG" .env; ln -sfn "$$UPRAVA_NODE_CONFIG" node.env; ln -sfn "$$UPRAVA_CORE_STATE_DIR" core-state

rollback: activate

deploy:
	@active="$$(sed -n 's/^UPRAVA_RELEASE_ID=//p' .env.release)"; \
	if [ "$$active" = candidate ]; then exit 42; fi; \
	printf '%s\n' "$$active" > deployed-release
MAKE

if RELEASE_ID=candidate DEPLOY_MODE=local INSTALL_DIR="$tmp" SUDO= \
    "$repo/scripts/deploy.sh" >/dev/null 2>&1; then
    echo "deploy rollback rehearsal accepted the failed candidate" >&2
    exit 1
fi
test "$(sed -n 's/^UPRAVA_RELEASE_ID=//p' "$tmp/.env.release")" = prior
test "$(cat "$tmp/deployed-release")" = prior
case "$(readlink "$tmp/current")" in
    */builds/releases/prior) ;;
    *) echo "automatic rollback did not restore current" >&2; exit 1 ;;
esac
test "$(readlink "$tmp/.env")" = prior-core.env
test "$(readlink "$tmp/node.env")" = prior-node.env
test "$(readlink "$tmp/core-state")" = state/prior
echo "Forced activation failure automatically restored the prior release"
