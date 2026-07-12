#!/usr/bin/env bash
set -Eeuo pipefail
CI_PHASE=build
source "$(dirname "$0")/lib.sh"

make_cmd=${MAKE:-make}
artifact_dir=${CI_ARTIFACT_DIR:-handoff}
release_id=${RELEASE_ID:-$(git rev-parse --short=12 HEAD)}
manifest=${RELEASE_MANIFEST:-builds/releases/${release_id}.env.release}
docker_config=${DOCKER_CONFIG:-$PWD/.docker}

cleanup_registry() {
  docker --config "$docker_config" logout ghcr.io >/dev/null 2>&1 || true
}
trap cleanup_registry EXIT

if [[ -n ${GHCR_TOKEN:-} ]]; then
  ci_set_stage registry-login
  install -d -m 700 "$docker_config"
  printf '%s' "$GHCR_TOKEN" | docker --config "$docker_config" login ghcr.io \
    --username "${GHCR_USER:?GHCR_USER is required}" --password-stdin >/dev/null
fi

ci_set_stage images
"$make_cmd" build RELEASE_ID="$release_id" RELEASE_MANIFEST="$manifest"

ci_set_stage startup
"$make_cmd" image-runtime RELEASE_ID="$release_id" RELEASE_MANIFEST="$manifest"

ci_set_stage publish
"$make_cmd" push RELEASE_ID="$release_id" RELEASE_MANIFEST="$manifest" DOCKER_CONFIG="$docker_config"

ci_set_stage handoff
test -s "$manifest"
mkdir -p "$artifact_dir"
install -m 644 "$manifest" "$artifact_dir/uprava-release.env"

ci_set_stage complete
ci_log "release=$release_id manifest=$artifact_dir/uprava-release.env"
