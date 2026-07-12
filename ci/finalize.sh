#!/usr/bin/env bash
set -Eeuo pipefail
CI_PHASE=finalize
source "$(dirname "$0")/lib.sh"

install_dir=${INSTALL_DIR:-/opt/apps/uprava}
manifest=${RELEASE_MANIFEST:-${CI_ARTIFACT_DIR:-handoff}/uprava-release.env}
sudo_cmd=${SUDO:-sudo}
retries=${FINALIZE_RETRIES:-60}
delay=${FINALIZE_DELAY_SECONDS:-2}
root_helper=${UPRAVA_ROOT_FINALIZE_HELPER:-/usr/local/sbin/uprava-ci-root-finalize}

if [[ ${UPRAVA_ROOT_PHASE:-0} != 1 ]]; then
  ci_set_stage root-contract
  sudo -n "$root_helper"
  ci_set_stage complete
  exit 0
fi

test -s "$manifest"
# shellcheck disable=SC1090
source "$manifest"
: "${UPRAVA_RELEASE_SHA:?missing release SHA}"
: "${UPRAVA_NODE_VERSION:?missing Node version}"
: "${UPRAVA_AUTO_APPROVE_NODE_NAME:?missing production Node name}"

domain=${UPRAVA_DOMAIN:-$("$sudo_cmd" awk -F= '$1 == "UPRAVA_DOMAIN" {print $2}' /etc/uprava/core.env)}
compose=("$sudo_cmd" docker compose --env-file /etc/uprava/core.env --env-file "$install_dir/.env.release" -f "$install_dir/compose.yaml")

wait_until() {
  local description=$1
  shift
  local attempt=1
  until "$@"; do
    if (( attempt >= retries )); then
      printf 'Timed out waiting for %s\n' "$description" >&2
      return 1
    fi
    sleep "$delay"
    ((attempt += 1))
  done
}

ci_set_stage core-web-health
wait_until core "${compose[@]}" exec -T core uprava-server healthcheck 127.0.0.1:8080
wait_until web "${compose[@]}" exec -T web wget -qO- http://127.0.0.1:8080/health

ci_set_stage public-release
wait_until public-health curl -fsS "https://${domain}/health"
version_json=$(curl -fsS "https://${domain}/api/v1/version")
python3 -c 'import json,sys; expected=sys.argv[1]; actual=json.load(sys.stdin).get("release_id"); raise SystemExit(0 if actual == expected else f"release mismatch: {actual}")' \
  "$UPRAVA_RELEASE_SHA" <<<"$version_json"

ci_set_stage node-readiness
wait_until node-service "$sudo_cmd" systemctl is-active --quiet uprava-node.service
wait_until node-heartbeat "${compose[@]}" exec -T core uprava-server deployment-status \
  "$UPRAVA_AUTO_APPROVE_NODE_NAME" "$UPRAVA_NODE_VERSION" 45

ci_set_stage retention
"$sudo_cmd" INSTALL_DIR="$install_dir" "$install_dir/scripts/prune-uprava-releases.sh"
"$sudo_cmd" "$install_dir/scripts/prune-uprava-images.sh"

ci_set_stage summary
printf 'release=%s core=healthy web=healthy node=%s node_version=%s\n' \
  "$UPRAVA_RELEASE_SHA" "$UPRAVA_AUTO_APPROVE_NODE_NAME" "$UPRAVA_NODE_VERSION"
