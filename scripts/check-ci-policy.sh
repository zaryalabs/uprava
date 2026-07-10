#!/bin/sh
set -eu

workflow="${CI_WORKFLOW:-.github/workflows/ci.yml}"

fail() {
  echo "CI policy check failed: $1" >&2
  exit 1
}

job_block() {
  job="$1"
  awk -v header="  ${job}:" '
    $0 == header { inside = 1 }
    inside && $0 != header && $0 ~ /^  [a-zA-Z0-9_-]+:$/ { exit }
    inside { print }
  ' "$workflow"
}

job_rejects_pattern() {
  job="$1"
  pattern="$2"
  description="$3"
  if job_block "$job" | grep -Eq -- "$pattern"; then
    fail "${job}: ${description}"
  fi
}

validate_job_shape() {
  job="$1"
  expected_keys="$2"
  job_block "$job" | awk -v job="$job" -v expected_keys="$expected_keys" '
    function invalid(message) {
      print "job syntax: " message > "/dev/stderr"
      failed = 1
      exit 1
    }

    BEGIN {
      count = split(expected_keys, keys, " ")
      for (i = 1; i <= count; i++) expected[keys[i]] = 1
    }

    NR == 1 {
      if ($0 != "  " job ":") invalid("missing canonical job header for " job)
      saw_job = 1
      next
    }

    /^    [^ ]/ {
      if ($0 ~ /^    #/) next
      entry = substr($0, 5)
      if (entry !~ /^[a-zA-Z0-9_-]+:/) {
        invalid(job " has non-canonical job-level key: " entry)
      }
      key = entry
      sub(/:.*/, "", key)
      if (!expected[key]) invalid(job " has unexpected job-level key: " key)
      if (seen[key]++) invalid(job " has duplicate job-level key: " key)
    }

    END {
      if (failed) exit 1
      if (!saw_job) invalid("job is missing: " job)
      for (key in expected) {
        if (seen[key] != 1) invalid(job " must declare canonical job-level key exactly once: " key)
      }
    }
  ' || fail "${job}: job-level key syntax or allow-list is invalid"
}

validate_job_scalar() {
  job="$1"
  key="$2"
  expected="$3"
  awk -v header="  ${job}:" -v key="$key" -v expected="$expected" '
    function trim(value) {
      sub(/^[[:space:]]+/, "", value)
      sub(/[[:space:]]+$/, "", value)
      return value
    }

    function normalize(value) {
      value = trim(value)
      gsub(/[[:space:]]+/, " ", value)
      return value
    }

    function invalid(message) {
      print key " policy: " message > "/dev/stderr"
      failed = 1
      exit 1
    }

    $0 == header {
      in_job = 1
      saw_job = 1
      next
    }

    in_job && $0 ~ /^  [a-zA-Z0-9_-]+:$/ { in_job = 0 }

    in_job {
      if ($0 ~ /^[[:space:]]*#/) next

      if (collecting) {
        if ($0 ~ /^[[:space:]]*$/) next
        if ($0 ~ /^      /) {
          part = trim($0)
          value = value (value == "" ? "" : " ") part
          next
        }
        collecting = 0
      }

      if (index($0, "    " key ":") == 1) {
        if (found++) invalid("duplicate canonical job-level key")
        value = trim(substr($0, length(key) + 6))
        if (value == ">-" || value == ">") {
          value = ""
          collecting = 1
        }
      }
    }

    END {
      if (failed) exit 1
      if (!saw_job) invalid("job is missing")
      if (found != 1) invalid("canonical job-level key must appear exactly once")
      actual = normalize(value)
      wanted = normalize(expected)
      if (actual != wanted) invalid("expected `" wanted "`, got `" actual "`")
    }
  ' "$workflow" || fail "${job}: actual job-level ${key} does not match policy"
}

validate_triggers() {
  awk '
    function invalid(message) {
      print "trigger policy: " message > "/dev/stderr"
      failed = 1
      exit 1
    }

    $0 == "on:" {
      if (saw_on) invalid("duplicate on block")
      saw_on = 1
      in_on = 1
      next
    }

    in_on && $0 ~ /^[^ ]/ { in_on = 0 }

    in_on {
      if ($0 ~ /^[[:space:]]*$/) next

      if ($0 ~ /^  [a-zA-Z0-9_-]+:$/) {
        event = substr($0, 3, length($0) - 3)
        if (event != "pull_request" && event != "push" && event != "workflow_dispatch") {
          invalid("unexpected event " event)
        }
        if (seen_event[event]++) invalid("duplicate event " event)
        next
      }

      if (event == "pull_request" || event == "push") {
        if ($0 == "    branches:") {
          if (seen_branches[event]++) invalid("duplicate branches for " event)
          next
        }
        if ($0 ~ /^      - /) {
          if (!seen_branches[event]) invalid("branch without branches key for " event)
          branch = substr($0, 9)
          if (branch != "main") invalid("unexpected " event " branch " branch)
          branch_count[event]++
          next
        }
      }

      invalid("unexpected entry in on block: " $0)
    }

    END {
      if (failed) exit 1
      if (!saw_on) invalid("missing on block")
      if (seen_event["pull_request"] != 1 || branch_count["pull_request"] != 1) {
        invalid("pull_request must target only main")
      }
      if (seen_event["push"] != 1 || branch_count["push"] != 1) {
        invalid("push must target only main")
      }
      if (seen_event["workflow_dispatch"] != 1) {
        invalid("workflow_dispatch must be declared once")
      }
    }
  ' "$workflow" || fail "event triggers do not match the allow-list"
}

validate_job_set() {
  actual="$(awk '
    $0 == "jobs:" { in_jobs = 1; next }
    in_jobs && $0 ~ /^  [a-zA-Z0-9_-]+:$/ {
      name = substr($0, 3, length($0) - 3)
      jobs = jobs (jobs == "" ? "" : " ") name
    }
    END { print jobs }
  ' "$workflow")"
  expected="check msrv stable release deploy"
  test "$actual" = "$expected" || fail "job allow-list changed: ${actual}"
}

validate_permissions() {
  job="$1"
  expected_packages="$2"
  job_block "$job" | awk -v expected_packages="$expected_packages" '
    function invalid() { failed = 1; exit 1 }

    $0 == "    permissions:" {
      if (saw_permissions) invalid()
      saw_permissions = 1
      in_permissions = 1
      next
    }

    in_permissions && $0 ~ /^    [^ ]/ { in_permissions = 0 }

    in_permissions {
      if ($0 ~ /^[[:space:]]*$/) next
      if ($0 !~ /^      [a-zA-Z0-9_-]+: [a-zA-Z]+$/) invalid()
      key = $1
      value = $2
      sub(/:$/, "", key)
      if (seen[key]++) invalid()
      if (key == "contents" && value == "read") contents_ok = 1
      else if (key == "packages" && value == expected_packages) packages_ok = 1
      else invalid()
      permission_count++
    }

    END {
      if (failed || !saw_permissions || permission_count != 2 || !contents_ok || !packages_ok) exit 1
    }
  ' || fail "${job}: permissions must be exactly contents: read and packages: ${expected_packages}"
}

step_block() {
  job="$1"
  step_name="$2"
  job_block "$job" | awk -v marker="      - name: ${step_name}" '
    /^      - / {
      active = ($0 == marker)
      if (active) print
      next
    }
    active { print }
  '
}

validate_step_once() {
  job="$1"
  step_name="$2"
  count="$(job_block "$job" | awk -v marker="      - name: ${step_name}" '$0 == marker { count++ } END { print count + 0 }')"
  test "$count" -eq 1 || fail "${job}: active step must appear exactly once: ${step_name}"
}

active_step_line() {
  job="$1"
  step_name="$2"
  job_block "$job" | awk -v marker="      - name: ${step_name}" '$0 == marker { print NR }'
}

step_requires_active() {
  job="$1"
  step_name="$2"
  pattern="$3"
  description="$4"
  step_block "$job" "$step_name" | awk '$0 !~ /^[[:space:]]*#/' | grep -Eq -- "$pattern" || fail "${job}/${step_name}: ${description}"
}

step_rejects_active() {
  job="$1"
  step_name="$2"
  pattern="$3"
  description="$4"
  if step_block "$job" "$step_name" | awk '$0 !~ /^[[:space:]]*#/' | grep -Eq -- "$pattern"; then
    fail "${job}/${step_name}: ${description}"
  fi
}

validate_step_allowlist() {
  job="$1"
  expected_steps="$2"
  job_block "$job" | awk -v job="$job" -v expected_steps="$expected_steps" '
    function invalid(message) {
      print "step policy: " job ": " message > "/dev/stderr"
      failed = 1
      exit 1
    }

    BEGIN { expected_count = split(expected_steps, expected, "[|]") }

    /^      - / {
      if ($0 !~ /^      - name: /) invalid("every step must start with a canonical name")
      actual_count++
      name = substr($0, 15)
      if (actual_count > expected_count || name != expected[actual_count]) {
        invalid("unexpected step " actual_count ": " name)
      }
    }

    END {
      if (failed) exit 1
      if (actual_count != expected_count) invalid("expected " expected_count " steps, got " actual_count)
    }
  ' || fail "${job}: step names/order must match the complete allow-list"
}

validate_step_working_directory() {
  job="$1"
  step_name="$2"
  expected='        working-directory: ${{ github.workspace }}'
  step_block "$job" "$step_name" | awk -v expected="$expected" '
    /^        working-directory:/ {
      count++
      if ($0 != expected) invalid = 1
    }
    END { if (count != 1 || invalid) exit 1 }
  ' || fail "${job}/${step_name}: working-directory must be the single stable GitHub workspace root"
}

validate_consumer_preamble() {
  job="$1"
  step_name="$2"
  step_block "$job" "$step_name" | awk -v job="$job" -v step_name="$step_name" '
    function invalid(message) {
      print "consumer preamble: " job "/" step_name ": " message > "/dev/stderr"
      failed = 1
      exit 1
    }

    BEGIN {
      expected[1] = "      - name: " step_name
      expected[2] = "        working-directory: ${{ github.workspace }}"
      expected[3] = "        run: |"
      expected[4] = "          test -n \"${GITHUB_WORKSPACE:-}\""
      expected[5] = "          test -n \"${GITHUB_RUN_ID:-}\""
      expected[6] = "          test -n \"${GITHUB_RUN_ATTEMPT:-}\""
      expected[7] = "          test -n \"${GITHUB_JOB:-}\""
      expected[8] = "          test \"${PWD}\" = \"${GITHUB_WORKSPACE}\""
      expected[9] = "          github_workspace_physical=\"$(pwd -P)\""
      expected[10] = "          workspace_root=\"${GITHUB_WORKSPACE}/.ci-workspace\""
      expected[11] = "          test ! -L \"${workspace_root}\""
      expected[12] = "          cd \"${workspace_root}\""
      expected[13] = "          workspace_root_physical=\"$(pwd -P)\""
      expected[14] = "          test \"${workspace_root_physical}\" = \"${github_workspace_physical}/.ci-workspace\""
      expected[15] = "          run_workspace_name=\"${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}-${GITHUB_JOB}\""
      expected[16] = "          test ! -L \"${run_workspace_name}\""
      expected[17] = "          cd \"${run_workspace_name}\""
      expected[18] = "          run_workspace=\"$(pwd -P)\""
      expected[19] = "          test \"${run_workspace}\" = \"${workspace_root_physical}/${run_workspace_name}\""
    }

    $0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/ { next }

    {
      active_count++
      if (active_count <= 19 && $0 != expected[active_count]) {
        invalid("unexpected active prefix line " active_count ": " $0)
      }
    }

    END {
      if (failed) exit 1
      if (active_count < 20) invalid("consumer has no command after the canonical preamble")
    }
  ' || fail "${job}/${step_name}: canonical physical-containment preamble is required before commands"
}

validate_cleanup_sequence() {
  job="$1"
  step_block "$job" "Clean persistent runner workspace" | awk -v job="$job" '
    function invalid(message) {
      print "cleanup policy: " job ": " message > "/dev/stderr"
      failed = 1
      exit 1
    }

    BEGIN {
      expected[1] = "      - name: Clean persistent runner workspace"
      expected[2] = "        run: |"
      expected[3] = "          test -n \"${GITHUB_WORKSPACE:-}\""
      expected[4] = "          test -n \"${GITHUB_RUN_ID:-}\""
      expected[5] = "          test -n \"${GITHUB_RUN_ATTEMPT:-}\""
      expected[6] = "          test -n \"${GITHUB_JOB:-}\""
      expected[7] = "          test \"${PWD}\" = \"${GITHUB_WORKSPACE}\""
      expected[8] = "          github_workspace_physical=\"$(pwd -P)\""
      expected[9] = "          workspace_root=\"${GITHUB_WORKSPACE}/.ci-workspace\""
      expected[10] = "          test ! -L \"${workspace_root}\""
      expected[11] = "          mkdir -p \"${workspace_root}\""
      expected[12] = "          test ! -L \"${workspace_root}\""
      expected[13] = "          cd \"${workspace_root}\""
      expected[14] = "          workspace_root_physical=\"$(pwd -P)\""
      expected[15] = "          test \"${workspace_root_physical}\" = \"${github_workspace_physical}/.ci-workspace\""
      expected[16] = "          find . -mindepth 1 -maxdepth 1 -type d -mtime +7 -exec rm -rf -- {} +"
      expected[17] = "          run_workspace_name=\"${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}-${GITHUB_JOB}\""
      expected[18] = "          test ! -L \"${run_workspace_name}\""
      expected[19] = "          mkdir \"${run_workspace_name}\""
      expected[20] = "          test ! -L \"${run_workspace_name}\""
      expected[21] = "          cd \"${run_workspace_name}\""
      expected[22] = "          run_workspace=\"$(pwd -P)\""
      expected[23] = "          test \"${run_workspace}\" = \"${workspace_root_physical}/${run_workspace_name}\""
      expected[24] = "          find . -mindepth 1 -maxdepth 1 -exec rm -rf -- {} +"
      expected[25] = "          git init ."
      expected[26] = "          git reset --hard HEAD 2>/dev/null || true"
      expected[27] = "          git clean -ffdx"
    }

    $0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/ { next }

    {
      active_count++
      if ($0 != expected[active_count]) {
        invalid("unexpected active line " active_count ": " $0)
      }
    }

    END {
      if (failed) exit 1
      if (active_count != 27) invalid("expected exactly 27 active lines, got " active_count)
    }
  ' || fail "${job}: cleanup step must match the canonical unique-workspace sequence"
}

validate_checkout_sequence() {
  job="$1"
  step_block "$job" "Checkout repository" | awk -v job="$job" '
    function invalid(message) {
      print "checkout policy: " job ": " message > "/dev/stderr"
      failed = 1
      exit 1
    }

    BEGIN {
      expected[1] = "      - name: Checkout repository"
      expected[2] = "        working-directory: ${{ github.workspace }}"
      expected[3] = "        run: |"
      expected[4] = "          test -n \"${GITHUB_WORKSPACE:-}\""
      expected[5] = "          test -n \"${GITHUB_RUN_ID:-}\""
      expected[6] = "          test -n \"${GITHUB_RUN_ATTEMPT:-}\""
      expected[7] = "          test -n \"${GITHUB_JOB:-}\""
      expected[8] = "          test \"${PWD}\" = \"${GITHUB_WORKSPACE}\""
      expected[9] = "          github_workspace_physical=\"$(pwd -P)\""
      expected[10] = "          workspace_root=\"${GITHUB_WORKSPACE}/.ci-workspace\""
      expected[11] = "          test ! -L \"${workspace_root}\""
      expected[12] = "          cd \"${workspace_root}\""
      expected[13] = "          workspace_root_physical=\"$(pwd -P)\""
      expected[14] = "          test \"${workspace_root_physical}\" = \"${github_workspace_physical}/.ci-workspace\""
      expected[15] = "          run_workspace_name=\"${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}-${GITHUB_JOB}\""
      expected[16] = "          test ! -L \"${run_workspace_name}\""
      expected[17] = "          cd \"${run_workspace_name}\""
      expected[18] = "          run_workspace=\"$(pwd -P)\""
      expected[19] = "          test \"${run_workspace}\" = \"${workspace_root_physical}/${run_workspace_name}\""
      expected[20] = "          git init ."
      expected[21] = "          git remote remove origin 2>/dev/null || true"
      expected[22] = "          git remote add origin \"https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git\""
      expected[23] = "          git fetch --depth=1 origin \"${CHECKOUT_REF}\""
      expected[24] = "          git checkout --force --detach FETCH_HEAD"
      expected[25] = "        env:"
      expected[26] = "          CHECKOUT_REF: ${{ github.ref }}"
      expected[27] = "          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}"
    }

    $0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/ { next }

    {
      active_count++
      if ($0 != expected[active_count]) invalid("unexpected active line " active_count ": " $0)
    }

    END {
      if (failed) exit 1
      if (active_count != 27) invalid("expected exactly 27 active lines, got " active_count)
    }
  ' || fail "${job}: checkout must use the canonical manual run-workspace sequence"
}

validate_prepare_payload() {
  step_block check "Prepare" | awk '
    BEGIN {
      expected[1] = "          rust_cache_dir=\"${PWD}/.ci-cache/rust\""
      expected[2] = "          mkdir -p \"${rust_cache_dir}/cargo\" \"${rust_cache_dir}/rustup\""
      expected[3] = "          web_run=\"docker run --rm --user $(id -u):$(id -g) -e npm_config_cache=/tmp/.npm -v ${PWD}/apps/web:/work -w /work node:24-bookworm-slim npm\""
      expected[4] = "          cargo_image=\"docker run --rm --user $(id -u):$(id -g) -e CARGO_HOME=/work/.cache/cargo -e CARGO_TARGET_DIR=/work/target -e RUSTUP_HOME=/work/.cache/rustup -v ${rust_cache_dir}:/work/.cache -v ${PWD}:/work -w /work rust:bookworm\""
      expected[5] = "          cargo_run=\"${cargo_image} cargo\""
      expected[6] = "          rustup_run=\"${cargo_image} rustup\""
      expected[7] = "          make web-install prepare WEB_RUN=\"${web_run}\" CARGO=\"${cargo_run}\" RUSTUP=\"${rustup_run}\" RUST_TOOLCHAIN=stable"
    }
    $0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/ { next }
    { active++; if (active > 19 && $0 != expected[active - 19]) exit 1 }
    END { if (active != 26) exit 1 }
  ' || fail "check/Prepare: payload must be the exact canonical seven-line prepare sequence"
}

validate_rust_check_payload() {
  job="$1"
  step_name="$2"
  expected_image="$3"
  step_block "$job" "$step_name" | awk -v job="$job" -v step_name="$step_name" -v expected_image="$expected_image" '
    BEGIN {
      expected[1] = "          cargo_image=\"docker run --rm --user $(id -u):$(id -g) -e CARGO_TARGET_DIR=/work/target -v ${PWD}:/work -w /work " expected_image "\""
      expected[2] = "          ${cargo_image} cargo check --workspace --all-targets --locked"
    }
    $0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/ { next }
    {
      active++
      if (active > 19 && $0 != expected[active - 19]) exit 1
    }
    END { if (active != 21) exit 1 }
  ' || fail "${job}/${step_name}: payload must be exactly the canonical image assignment then locked cargo check"
}

validate_release_payload() {
  step_block release "Build and publish release" | awk '
    BEGIN {
      expected[1] = "          make build"
      expected[2] = "          make push"
    }
    $0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/ { next }
    { active++; if (active > 19 && $0 != expected[active - 19]) exit 1 }
    END { if (active != 21) exit 1 }
  ' || fail "release/Build and publish release: payload must be exactly make build then make push"
}

validate_login_payload() {
  job="$1"
  step_block "$job" "Login to GHCR" | awk '
    BEGIN { expected = "          echo \"${{ secrets.GITHUB_TOKEN }}\" | docker login ghcr.io -u \"${GITHUB_REPOSITORY_OWNER}\" --password-stdin" }
    $0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/ { next }
    { active++; if (active == 20 && $0 != expected) exit 1 }
    END { if (active != 20) exit 1 }
  ' || fail "${job}/Login to GHCR: payload must be the single canonical registry login command"
}

validate_deploy_payload() {
  step_block deploy "Deploy current HEAD release" | awk '
    BEGIN {
      expected[1] = "          RELEASE_ID=\"$(git rev-parse --short=12 HEAD)\""
      expected[2] = "          make release-manifest RELEASE_ID=\"${RELEASE_ID}\""
      expected[3] = "          make install-ops INSTALL_DIR=/opt/apps/uprava SUDO=sudo"
      expected[4] = "          make install-release-manifest DEPLOY_HOST=zsa INSTALL_DIR=/opt/apps/uprava SUDO=sudo RELEASE_ID=\"${RELEASE_ID}\""
      expected[5] = "          make deploy DEPLOY_HOST=zsa INSTALL_DIR=/opt/apps/uprava SUDO=sudo RELEASE_ID=\"${RELEASE_ID}\" DEPLOY_MODE=local"
    }
    $0 ~ /^[[:space:]]*$/ || $0 ~ /^[[:space:]]*#/ { next }
    { active++; if (active > 19 && $0 != expected[active - 19]) exit 1 }
    END { if (active != 24) exit 1 }
  ' || fail "deploy/Deploy current HEAD release: payload must match the exact canonical five-line deployment sequence"
}

validate_single_active_assignment() {
  job="$1"
  step_name="$2"
  variable="$3"
  expected_pattern="$4"
  count="$(step_block "$job" "$step_name" | awk -v variable="$variable" '
    $0 !~ /^[[:space:]]*#/ && $0 ~ ("(^|[[:space:]])" variable "[[:space:]]*=") { count++ }
    END { print count + 0 }
  ')"
  test "$count" -eq 1 || fail "${job}/${step_name}: ${variable} must have exactly one active assignment"
  step_requires_active "$job" "$step_name" "$expected_pattern" "active ${variable} assignment does not match policy"
}

validate_single_active_cargo_check() {
  job="$1"
  step_name="$2"
  expected_pattern="$3"
  count="$(step_block "$job" "$step_name" | awk '
    $0 !~ /^[[:space:]]*#/ && $0 ~ /\$\{cargo_image\}[[:space:]]+cargo[[:space:]]+check([[:space:]]|$)/ { count++ }
    END { print count + 0 }
  ')"
  test "$count" -eq 1 || fail "${job}/${step_name}: cargo check must appear exactly once as active code"
  step_requires_active "$job" "$step_name" "$expected_pattern" "active cargo check must end with required --locked"
}

reject_active_before_line() {
  job="$1"
  limit="$2"
  pattern="$3"
  description="$4"
  if job_block "$job" | awk -v limit="$limit" 'NR < limit && $0 !~ /^[[:space:]]*#/ { print }' | grep -Eq -- "$pattern"; then
    fail "${job}: ${description}"
  fi
}

test -s "$workflow" || fail "workflow is missing"

validate_triggers
validate_job_set

if grep -Fq -- "pull_request_target" "$workflow"; then
  fail "pull_request_target must not be used"
fi
if grep -Eq '^permissions:' "$workflow"; then
  fail "workflow-wide permissions are forbidden"
fi
if awk '$0 !~ /^[[:space:]]*#/ { print }' "$workflow" | grep -Eq 'uses:[[:space:]]+actions/checkout(@|[[:space:]]|$)'; then
  fail "actions/checkout is forbidden; manual checkout must honor the canonical working-directory"
fi

for job in check msrv stable; do
  validate_job_shape "$job" "if permissions runs-on steps"
  validate_job_scalar "$job" if "github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository"
  validate_job_scalar "$job" runs-on "[self-hosted, zarya-main, geo-eu, ci]"
  validate_permissions "$job" read
done

for job in release deploy; do
  validate_job_shape "$job" "needs if permissions runs-on steps"
  validate_job_scalar "$job" runs-on "[self-hosted, zarya-main, geo-eu, ci]"
done
validate_permissions release write
validate_permissions deploy read
validate_job_scalar release needs "[check, msrv, stable]"
validate_job_scalar release if "github.event_name == 'push' || github.event_name == 'workflow_dispatch'"
validate_job_scalar deploy needs "release"
validate_job_scalar deploy if "github.event_name == 'workflow_dispatch'"

publish_pattern='(^|[[:space:]])(make([[:space:]]+[^[:space:]]+)*[[:space:]]+(push|publish)|docker([[:space:]]+[^[:space:]]+)*[[:space:]]+push|docker[[:space:]]+buildx[[:space:]]+build.*--push|(cargo|npm|pnpm)[[:space:]]+publish|gh[[:space:]]+release[[:space:]]+create)([[:space:]]|$)'
deploy_pattern='(^|[[:space:]])(make([[:space:]]+[^[:space:]]+)*[[:space:]]+(deploy|install-ops|install-release-manifest)|scripts/deploy\.sh)([[:space:]]|$)'
login_pattern='(^|[[:space:]])docker([[:space:]]+[^[:space:]]+)*[[:space:]]+login([[:space:]]|$)'

for job in check msrv stable deploy; do
  job_rejects_pattern "$job" "$publish_pattern" "publishing commands are allowed only in release"
done
for job in check msrv stable release; do
  job_rejects_pattern "$job" "$deploy_pattern" "deployment commands are allowed only in deploy"
done
for job in check msrv stable; do
  job_rejects_pattern "$job" "$login_pattern" "registry login is forbidden in check jobs"
done

for job in check msrv stable release deploy; do
  validate_step_once "$job" "Clean persistent runner workspace"
  validate_step_once "$job" "Checkout repository"
  clean_line="$(active_step_line "$job" "Clean persistent runner workspace")"
  checkout_line="$(active_step_line "$job" "Checkout repository")"
  test "$clean_line" -lt "$checkout_line" || fail "${job}: cleanup must precede checkout"
  reject_active_before_line "$job" "$clean_line" '(^|[[:space:]:;|&])git[[:space:]]+checkout([[:space:]]|$)|uses:[[:space:]]+actions/checkout(@|[[:space:]]|$)' "active checkout command must not run before cleanup"
  validate_cleanup_sequence "$job"
  validate_checkout_sequence "$job"
  rm_rf_count="$(job_block "$job" | awk '$0 !~ /^[[:space:]]*#/ && $0 ~ "(^|[[:space:]:;|&])(/bin/)?rm[[:space:]]+-rf([[:space:]]|$)" { count++ } END { print count + 0 }')"
  test "$rm_rf_count" -eq 2 || fail "${job}: active rm -rf is allowed only in the two canonical find cleanup commands"
done

validate_step_allowlist check "Clean persistent runner workspace|Checkout repository|Prepare"
validate_step_allowlist msrv "Clean persistent runner workspace|Checkout repository|Check Rust 1.88 MSRV"
validate_step_allowlist stable "Clean persistent runner workspace|Checkout repository|Check current stable Rust"
validate_step_allowlist release "Clean persistent runner workspace|Checkout repository|Login to GHCR|Build and publish release"
validate_step_allowlist deploy "Clean persistent runner workspace|Checkout repository|Login to GHCR|Deploy current HEAD release"

validate_step_working_directory check "Checkout repository"
validate_step_working_directory check "Prepare"
validate_step_working_directory msrv "Checkout repository"
validate_step_working_directory msrv "Check Rust 1.88 MSRV"
validate_step_working_directory stable "Checkout repository"
validate_step_working_directory stable "Check current stable Rust"
validate_step_working_directory release "Checkout repository"
validate_step_working_directory release "Login to GHCR"
validate_step_working_directory release "Build and publish release"
validate_step_working_directory deploy "Checkout repository"
validate_step_working_directory deploy "Login to GHCR"
validate_step_working_directory deploy "Deploy current HEAD release"

validate_consumer_preamble check "Checkout repository"
validate_consumer_preamble check "Prepare"
validate_consumer_preamble msrv "Checkout repository"
validate_consumer_preamble msrv "Check Rust 1.88 MSRV"
validate_consumer_preamble stable "Checkout repository"
validate_consumer_preamble stable "Check current stable Rust"
validate_consumer_preamble release "Checkout repository"
validate_consumer_preamble release "Login to GHCR"
validate_consumer_preamble release "Build and publish release"
validate_consumer_preamble deploy "Checkout repository"
validate_consumer_preamble deploy "Login to GHCR"
validate_consumer_preamble deploy "Deploy current HEAD release"

validate_prepare_payload
validate_rust_check_payload msrv "Check Rust 1.88 MSRV" "rust:1.88-bookworm"
validate_rust_check_payload stable "Check current stable Rust" "rust:bookworm"
validate_release_payload
validate_login_payload release
validate_login_payload deploy
validate_deploy_payload

termination_pattern='(^|[[:space:];|&])(exit|return|false)([[:space:];|&]|$)'
control_flow_pattern='(^|[[:space:];|&])(if|then|else|fi|for|while|until|case|esac)([[:space:];|&]|$)|<<'

validate_step_once check "Prepare"
step_rejects_active check "Prepare" "$termination_pattern" "early-termination commands are forbidden in the check step"
step_rejects_active check "Prepare" "$control_flow_pattern" "shell control-flow and heredocs are forbidden in the check step"
validate_single_active_assignment check "Prepare" cargo_image '^[[:space:]]+cargo_image="docker run .* rust:bookworm"[[:space:]]*$'

validate_step_once msrv "Check Rust 1.88 MSRV"
step_rejects_active msrv "Check Rust 1.88 MSRV" "$termination_pattern" "early-termination commands are forbidden in the MSRV step"
step_rejects_active msrv "Check Rust 1.88 MSRV" "$control_flow_pattern" "shell control-flow and heredocs are forbidden in the MSRV step"
validate_single_active_assignment msrv "Check Rust 1.88 MSRV" cargo_image '^[[:space:]]+cargo_image="docker run .* rust:1\.88-bookworm"[[:space:]]*$'
validate_single_active_cargo_check msrv "Check Rust 1.88 MSRV" '^[[:space:]]+\$\{cargo_image\} cargo check --workspace --all-targets --locked[[:space:]]*$'

validate_step_once stable "Check current stable Rust"
step_rejects_active stable "Check current stable Rust" "$termination_pattern" "early-termination commands are forbidden in the stable step"
step_rejects_active stable "Check current stable Rust" "$control_flow_pattern" "shell control-flow and heredocs are forbidden in the stable step"
validate_single_active_assignment stable "Check current stable Rust" cargo_image '^[[:space:]]+cargo_image="docker run .* rust:bookworm"[[:space:]]*$'
validate_single_active_cargo_check stable "Check current stable Rust" '^[[:space:]]+\$\{cargo_image\} cargo check --workspace --all-targets --locked[[:space:]]*$'

validate_step_once release "Build and publish release"
step_requires_active release "Build and publish release" '^[[:space:]]+make push[[:space:]]*$' "active publishing command is missing"

validate_step_once deploy "Deploy current HEAD release"
step_requires_active deploy "Deploy current HEAD release" '^[[:space:]]+make deploy[[:space:]]+.*$' "active deployment command is missing"

if grep -Eiq 'playwright|web-e2e' "$workflow"; then
  fail "the final Playwright gate is not enabled in this unit"
fi

echo "CI workflow policy is valid"
