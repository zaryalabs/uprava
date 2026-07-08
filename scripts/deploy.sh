#!/usr/bin/env bash
set -euo pipefail

deploy_host="${DEPLOY_HOST:-zsa}"
deploy_mode="${DEPLOY_MODE:-ssh}"
install_dir="${INSTALL_DIR:-/opt/apps/uprava}"
release_id="${RELEASE_ID:-}"
sudo_cmd="${SUDO:-}"

if [[ -z "$release_id" ]]; then
  printf "RELEASE_ID is required\n" >&2
  exit 1
fi

run_local() {
  cd "$install_dir"
  if [[ -n "$sudo_cmd" ]]; then
    $sudo_cmd make activate RELEASE="$release_id" SUDO=
    $sudo_cmd make deploy SUDO=
  else
    make activate RELEASE="$release_id"
    make deploy
  fi
}

run_ssh() {
  ssh "$deploy_host" "cd '$install_dir' && make activate RELEASE='$release_id' && make deploy"
}

case "$deploy_mode" in
  local)
    run_local
    ;;
  ssh)
    run_ssh
    ;;
  *)
    printf "Unknown DEPLOY_MODE: %s\n" "$deploy_mode" >&2
    exit 1
    ;;
esac
