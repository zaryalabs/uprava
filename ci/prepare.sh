#!/usr/bin/env bash
set -Eeuo pipefail
CI_PHASE=prepare
source "$(dirname "$0")/lib.sh"

make_cmd=${MAKE:-make}

ci_set_stage dependencies
"$make_cmd" web-install

ci_set_stage source-checks
if [[ ${CI_MAIN:-0} == 1 ]]; then
  "$make_cmd" push-check
else
  "$make_cmd" docs-l protocol-check rust-l rust-t web-l web-t generated-ui-t web-dl scripts-check
fi

ci_set_stage complete
ci_log passed
