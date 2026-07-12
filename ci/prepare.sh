#!/usr/bin/env bash
set -Eeuo pipefail
CI_PHASE=prepare
source "$(dirname "$0")/lib.sh"

make_cmd=${MAKE:-make}

ci_set_stage dependencies
"$make_cmd" web-install

ci_set_stage source-checks
"$make_cmd" docs-l protocol-check rust-l rust-t web-l web-t web-dl scripts-check

if [[ ${CI_MAIN:-0} == 1 ]]; then
  ci_set_stage msrv
  cargo +1.88.0 check --workspace --all-targets --locked

  ci_set_stage audits
  "$make_cmd" rust-dl

  ci_set_stage browser
  "$make_cmd" web-e2e
fi

ci_set_stage complete
ci_log passed
