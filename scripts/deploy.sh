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
  local prior_release
  prior_release="$(sed -n 's/^UPRAVA_RELEASE_ID=//p' .env.release | head -n 1)"
  if [[ -z "$prior_release" ]]; then
    printf "Active release id is unavailable; refusing non-rollbackable deploy\n" >&2
    return 1
  fi
  if [[ -n "$sudo_cmd" ]]; then
    $sudo_cmd make activate RELEASE="$release_id" SUDO=
    if ! $sudo_cmd make deploy SUDO=; then
      printf "Activation failed; rolling back %s -> %s\n" "$release_id" "$prior_release" >&2
      $sudo_cmd make rollback RELEASE="$prior_release" SUDO=
      $sudo_cmd make deploy SUDO=
      return 1
    fi
  else
    make activate RELEASE="$release_id"
    if ! make deploy; then
      printf "Activation failed; rolling back %s -> %s\n" "$release_id" "$prior_release" >&2
      make rollback RELEASE="$prior_release"
      make deploy
      return 1
    fi
  fi
}

run_ssh() {
  ssh "$deploy_host" "set -eu; cd '$install_dir'; prior=\$(sed -n 's/^UPRAVA_RELEASE_ID=//p' .env.release | head -n 1); test -n \"\$prior\"; make activate RELEASE='$release_id'; if ! make deploy; then echo 'Activation failed; rolling back' >&2; make rollback RELEASE=\"\$prior\"; make deploy; exit 1; fi"
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
