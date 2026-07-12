#!/usr/bin/env bash
set -Eeuo pipefail

phase=${1:?usage: ci/run.sh <prepare|build|deploy|finalize|cleanup> [phase]}
source_dir=$(git rev-parse --show-toplevel)
root=${CI_WORKSPACE_ROOT:-${GITHUB_WORKSPACE:-$source_dir}/.ci-workspace}
run_key=${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}
target_phase=${2:-$phase}
workspace="$root/$run_key-$target_phase"

cleanup_workspace() {
  git -C "$source_dir" worktree remove --force "$workspace" >/dev/null 2>&1 || rm -rf "$workspace"
}

if [[ $phase == cleanup ]]; then
  cleanup_workspace
  exit 0
fi

case "$phase" in prepare|build|deploy|finalize) ;; *) echo "unknown phase: $phase" >&2; exit 2 ;; esac

test ! -L "$root"
mkdir -p "$root"
find "$root" -mindepth 1 -maxdepth 1 -type d -mmin +360 -exec rm -rf -- {} +
available_kb=$(df -Pk "$root" | awk 'NR == 2 {print $4}')
test "$available_kb" -ge "${CI_MIN_AVAILABLE_KB:-10485760}"
cleanup_workspace
git -C "$source_dir" worktree add --force --detach "$workspace" "${GITHUB_SHA:-HEAD}" >/dev/null
trap cleanup_workspace EXIT INT TERM

artifact_dir=${CI_ARTIFACT_DIR:-${GITHUB_WORKSPACE:-$source_dir}/handoff}
mkdir -p "$artifact_dir"

if [[ $phase == prepare ]]; then
  image="uprava-ci:${GITHUB_SHA:-local}"
  docker build -t "$image" -f "$workspace/ci/Dockerfile" "$workspace"
  cache="$root/cache"
  mkdir -p "$cache/cargo" "$cache/npm" "$cache/target" "$cache/tmp"
  docker run --rm --user "$(id -u):$(id -g)" \
    -e HOME=/tmp -e CI_MAIN="${CI_MAIN:-0}" \
    -e CARGO_HOME=/cache/cargo -e CARGO_TARGET_DIR=/cache/target -e npm_config_cache=/cache/npm \
    -e RUSTUP_HOME=/usr/local/rustup \
    -v "$workspace:/work" -v "$cache:/cache" -v "$cache/tmp:/tmp" \
    -w /work "$image" ci/prepare.sh
else
  (cd "$workspace" && CI_ARTIFACT_DIR="$artifact_dir" "ci/$phase.sh")
fi
