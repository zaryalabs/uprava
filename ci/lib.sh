#!/usr/bin/env bash
set -Eeuo pipefail

ci_phase=${CI_PHASE:-unknown}
ci_stage=bootstrap

ci_log() {
  printf '[ci] phase=%s stage=%s %s\n' "$ci_phase" "$ci_stage" "$*"
}

ci_set_stage() {
  ci_stage=$1
  ci_log start
}

ci_error() {
  local code=$?
  printf '[ci] phase=%s stage=%s command=%q exit=%s\n' \
    "$ci_phase" "$ci_stage" "$BASH_COMMAND" "$code" >&2
  exit "$code"
}

trap ci_error ERR
