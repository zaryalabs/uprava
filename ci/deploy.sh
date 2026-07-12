#!/usr/bin/env bash
set -Eeuo pipefail
CI_PHASE=deploy
source "$(dirname "$0")/lib.sh"

make_cmd=${MAKE:-make}
install_dir=${INSTALL_DIR:-/opt/apps/uprava}
manifest=${RELEASE_MANIFEST:-${CI_ARTIFACT_DIR:-handoff}/uprava-release.env}
sudo_cmd=${SUDO:-sudo}
docker_config=${DOCKER_CONFIG:-${CI_ARTIFACT_DIR:-$PWD}/.docker}
root_helper=${UPRAVA_ROOT_DEPLOY_HELPER:-/usr/local/sbin/uprava-ci-root-deploy}

cleanup_registry() {
  "$sudo_cmd" docker --config "$docker_config" logout ghcr.io >/dev/null 2>&1 || true
  "$sudo_cmd" rm -rf "$docker_config" >/dev/null 2>&1 || true
}
trap cleanup_registry EXIT

if [[ ${UPRAVA_ROOT_PHASE:-0} != 1 ]]; then
  test -s "$manifest"
  if [[ -n ${GHCR_TOKEN:-} ]]; then
    ci_set_stage registry-login
    install -d -m 700 "$docker_config"
    printf '%s' "$GHCR_TOKEN" | docker --config "$docker_config" login ghcr.io \
      --username "${GHCR_USER:?GHCR_USER is required}" --password-stdin >/dev/null
  fi
  ci_set_stage root-contract
  sudo -n "$root_helper"
  ci_set_stage complete
  exit 0
fi

test -s "$manifest"
release_id=$(sed -n 's/^UPRAVA_RELEASE_ID=//p' "$manifest")
test -n "$release_id"

ci_set_stage bootstrap
"$make_cmd" install-ops INSTALL_DIR="$install_dir" SUDO="$sudo_cmd"

ci_set_stage manifest
"$make_cmd" install-release-manifest INSTALL_DIR="$install_dir" SUDO="$sudo_cmd" \
  RELEASE_ID="$release_id" RELEASE_MANIFEST="$manifest"

ci_set_stage activate
"$sudo_cmd" "$make_cmd" -C "$install_dir" --no-print-directory deploy \
  RELEASE="$release_id" SUDO= DOCKER_CONFIG="$docker_config"

ci_set_stage complete
ci_log "release=$release_id"
