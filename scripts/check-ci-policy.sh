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

require_job() {
    job_block "$1" | grep -Fq -- "$2" || fail "$1: $3"
}

test -s "$workflow" || fail "workflow is missing"
grep -Fq '  pull_request:' "$workflow" || fail "pull_request trigger is required"
grep -Fq '  push:' "$workflow" || fail "push trigger is required"
! grep -Eq 'workflow_dispatch|pull_request_target' "$workflow" || fail "manual or privileged PR triggers are forbidden"
! grep -Fq 'run: |' "$workflow" || fail "workflow orchestration must live in ci scripts"

jobs=$(awk '/^jobs:$/ {seen=1; next} seen && /^  [a-zA-Z0-9_-]+:$/ {gsub(/^  |:$/, ""); printf "%s%s", sep, $0; sep=" "} END {print ""}' "$workflow")
test "$jobs" = 'prepare build deploy finalize' || fail "expected four phases, found: $jobs"

require_job prepare 'run: ci/run.sh prepare' "must dispatch the prepare script"
require_job build 'needs: prepare' "must depend on prepare"
require_job build 'run: ci/run.sh build' "must dispatch the build script"
require_job deploy 'needs: build' "must depend on build"
require_job deploy 'run: ci/run.sh deploy' "must dispatch the deploy script"
require_job finalize 'needs: deploy' "must depend on deploy"
require_job finalize 'run: ci/run.sh finalize' "must dispatch the finalize script"
for job in build deploy finalize; do
    require_job "$job" "if: github.event_name == 'push'" "must run only for main pushes"
done

require_job build 'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02' "must upload the immutable manifest"
require_job deploy 'actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093' "must download the build manifest"
require_job build 'packages: write' "requires registry publication permission"
if job_block prepare | grep -Fq 'packages: write'; then
    fail "prepare may not publish packages"
fi

uses=$(sed -n 's/^[[:space:]]*- uses: //p' "$workflow")
printf '%s\n' "$uses" | grep -Ev '^actions/(checkout|upload-artifact|download-artifact)@[0-9a-f]{40}( # .*)?$' >/dev/null &&
    fail "standard actions must be pinned to full commit SHAs"

grep -Fq 'cancel-in-progress: ${{ github.event_name == '\''pull_request'\'' }}' "$workflow" ||
    fail "only superseded pull requests may be cancelled"
test -s ci/Dockerfile || fail "ci/Dockerfile is missing"
! job_block prepare | grep -Fq '/var/run/docker.sock' || fail "prepare may not mount the Docker socket"

echo "CI workflow policy is valid"
