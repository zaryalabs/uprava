#!/bin/sh
set -eu

workflow=${1:-.github/workflows/ci.yml}

fail() {
    echo "CI policy check failed: $1" >&2
    exit 1
}

job_block() {
    job=$1
    awk -v header="  ${job}:" '
        $0 == header { inside = 1 }
        inside && $0 != header && $0 ~ /^  [a-zA-Z0-9_-]+:$/ { exit }
        inside { print }
    ' "$workflow"
}

require_job_text() {
    job=$1
    text=$2
    description=$3
    job_block "$job" | grep -Fq -- "$text" || fail "$job: $description"
}

reject_job_pattern() {
    job=$1
    pattern=$2
    description=$3
    if job_block "$job" | grep -Eq -- "$pattern"; then
        fail "$job: $description"
    fi
}

test -s "$workflow" || fail "workflow is missing"

grep -Fq '  pull_request:' "$workflow" || fail "pull_request trigger is required"
grep -Fq '  push:' "$workflow" || fail "push trigger is required"
if grep -Fq 'workflow_dispatch' "$workflow"; then
    fail "manual production workflow_dispatch is forbidden"
fi
if grep -Fq 'pull_request_target' "$workflow"; then
    fail "pull_request_target is forbidden"
fi

actual_jobs=$(awk '
    $0 == "jobs:" { in_jobs = 1; next }
    in_jobs && $0 ~ /^  [a-zA-Z0-9_-]+:$/ {
        name = substr($0, 3, length($0) - 3)
        jobs = jobs (jobs == "" ? "" : " ") name
    }
    END { print jobs }
' "$workflow")
test "$actual_jobs" = "check msrv stable delivery" ||
    fail "job allow-list changed: $actual_jobs"

grep -Fq '  cancel-in-progress: ${{ github.event_name == '\''pull_request'\'' }}' "$workflow" ||
    fail "only superseded pull-request workflows may be cancelled"

for job in check msrv stable delivery; do
    require_job_text "$job" '- name: Prepare bounded workspace' "bounded workspace preparation is required"
    require_job_text "$job" 'find "${root}" -mindepth 1 -maxdepth 1 -type d -mmin +360 -exec rm -rf -- {} +' "aged orphaned workspace GC is required"
    require_job_text "$job" 'test "${available_kb}" -ge 10485760' "10 GiB disk preflight is required"
    require_job_text "$job" '- name: Checkout repository' "manual checkout is required"
    require_job_text "$job" '- name: Cleanup' "final cleanup step is required"
    require_job_text "$job" 'if: always()' "cleanup must run unconditionally"
    require_job_text "$job" 'rm -rf "${RUN_WORKSPACE}"' "run workspace must be removed"
done

for job in check msrv stable; do
    reject_job_pattern "$job" 'docker[[:space:]]+(--config[[:space:]]+[^[:space:]]+[[:space:]]+)?(login|push)|make[[:space:]]+(push|deploy|install-ops|install-release-manifest)' "PR/check jobs may not publish or deploy"
done

require_job_text delivery "needs: [check, msrv, stable]" "delivery must depend on every check"
require_job_text delivery "if: github.event_name == 'push'" "delivery must run automatically only for main pushes"
require_job_text delivery 'packages: write' "delivery requires package publishing permission"
require_job_text delivery 'DOCKER_CONFIG:' "registry credentials must use a temporary Docker config"
require_job_text delivery 'make push' "delivery must publish immutable images and manifest"
require_job_text delivery 'make install-ops INSTALL_DIR=/opt/apps/uprava SUDO=sudo' "delivery must install reviewed ops assets"
require_job_text delivery 'make install-release-manifest INSTALL_DIR=/opt/apps/uprava SUDO=sudo RELEASE_ID="${RELEASE_ID}"' "delivery must install the generated manifest"
require_job_text delivery 'make deploy INSTALL_DIR=/opt/apps/uprava SUDO=sudo RELEASE_ID="${RELEASE_ID}" DEPLOY_MODE=local' "delivery must automatically activate main"
require_job_text delivery 'docker logout ghcr.io' "delivery must remove registry credentials"

if grep -Eq 'uses:[[:space:]]+actions/checkout' "$workflow"; then
    fail "actions/checkout is forbidden on the persistent privileged runner"
fi

echo "CI workflow policy is valid"
