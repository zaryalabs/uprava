#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
mkdir -p "$tmp/bin" "$tmp/install/scripts" "$tmp/handoff"
calls="$tmp/calls"

cat >"$tmp/bin/make" <<'SH'
#!/bin/sh
printf 'make %s\n' "$*" >>"$CALLS"
manifest=
for argument in "$@"; do
    case "$argument" in RELEASE_MANIFEST=*) manifest=${argument#*=} ;; esac
done
case " $* " in
    *' push '*)
        mkdir -p "$(dirname "$manifest")"
        cat >"$manifest" <<'EOF'
UPRAVA_RELEASE_ID=test
UPRAVA_RELEASE_SHA=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
UPRAVA_NODE_VERSION=0.2.3
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server'
UPRAVA_TOOLHIVE_PROFILE=toolhive
UPRAVA_TOOLHIVE_VERSION=0.40.0
UPRAVA_TOOLHIVE_CONFIG=/etc/uprava/toolhive.env
EOF
        ;;
esac
SH
cat >"$tmp/bin/cargo" <<'SH'
#!/bin/sh
printf 'cargo %s\n' "$*" >>"$CALLS"
SH
cat >"$tmp/bin/git" <<'SH'
#!/bin/sh
printf '%s\n' test
SH
cat >"$tmp/bin/docker" <<'SH'
#!/bin/sh
printf 'docker %s\n' "$*" >>"$CALLS"
case " $* " in
    *' exec -T toolhive thv version '*) printf '%s\n' 'ToolHive 0.40.0' ;;
esac
exit 0
SH
cat >"$tmp/bin/sudo" <<'SH'
#!/bin/sh
case "$1" in *=*) exec env "$@" ;; *) exec "$@" ;; esac
SH
cat >"$tmp/bin/curl" <<'SH'
#!/bin/sh
case "$*" in */api/v1/version*) printf '%s\n' '{"release_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}' ;; *) printf '%s\n' ok ;; esac
SH
cat >"$tmp/bin/systemctl" <<'SH'
#!/bin/sh
exit 0
SH
chmod 755 "$tmp/bin/"*

CALLS="$calls" PATH="$tmp/bin:$PATH" MAKE="$tmp/bin/make" CI_MAIN=0 bash "$repo/ci/prepare.sh" >/dev/null
grep -q '^make docs-l protocol-check rust-l rust-t web-l web-t web-dl scripts-check$' "$calls"
CALLS="$calls" PATH="$tmp/bin:$PATH" MAKE="$tmp/bin/make" CI_MAIN=1 bash "$repo/ci/prepare.sh" >/dev/null
grep -q '^make push-check$' "$calls"

grep -q 'entry: make l' "$repo/.pre-commit-config.yaml"
grep -A4 'entry: make l' "$repo/.pre-commit-config.yaml" | grep -q 'stages: \[pre-commit\]'
! grep -q 'entry: make t' "$repo/.pre-commit-config.yaml"
grep -q 'entry: make push-check' "$repo/.pre-commit-config.yaml"
grep -A5 'entry: make push-check' "$repo/.pre-commit-config.yaml" | grep -q 'stages: \[pre-push\]'
grep -q 'pre-commit install --hook-type pre-commit --hook-type pre-push' "$repo/Makefile"

CALLS="$calls" PATH="$tmp/bin:$PATH" MAKE="$tmp/bin/make" CI_ARTIFACT_DIR="$tmp/handoff" \
    RELEASE_MANIFEST="$tmp/release.env" DOCKER_CONFIG="$tmp/docker-config" bash "$repo/ci/build.sh" >/dev/null
test -s "$tmp/handoff/uprava-release.env"

cat >"$tmp/manifest.env" <<'EOF'
UPRAVA_RELEASE_ID=test
UPRAVA_RELEASE_SHA=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
UPRAVA_NODE_VERSION=0.2.3
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server'
UPRAVA_TOOLHIVE_PROFILE=toolhive
UPRAVA_TOOLHIVE_VERSION=0.40.0
UPRAVA_TOOLHIVE_CONFIG=/etc/uprava/toolhive.env
EOF
CALLS="$calls" PATH="$tmp/bin:$PATH" MAKE="$tmp/bin/make" SUDO="$tmp/bin/sudo" \
    UPRAVA_ROOT_PHASE=1 INSTALL_DIR="$tmp/install" RELEASE_MANIFEST="$tmp/manifest.env" \
    bash "$repo/ci/deploy.sh" >/dev/null
grep -q 'make install-ops' "$calls"
grep -q 'make -C .* deploy RELEASE=test' "$calls"

cat >"$tmp/install/scripts/prune-uprava-releases.sh" <<'SH'
#!/bin/sh
exit 0
SH
cp "$tmp/install/scripts/prune-uprava-releases.sh" "$tmp/install/scripts/prune-uprava-images.sh"
chmod 755 "$tmp/install/scripts/"*
CALLS="$calls" PATH="$tmp/bin:$PATH" SUDO="$tmp/bin/sudo" INSTALL_DIR="$tmp/install" \
    UPRAVA_ROOT_PHASE=1 RELEASE_MANIFEST="$tmp/manifest.env" UPRAVA_DOMAIN=example.test FINALIZE_RETRIES=1 \
    bash "$repo/ci/finalize.sh" >/dev/null

grep -q 'sudo -n.*root_helper' "$repo/ci/deploy.sh"
grep -q 'sudo -n.*root_helper' "$repo/ci/finalize.sh"
grep -q 'ls-remote.*refs/heads/main' "$repo/ops/uprava-ci-root"

echo "Focused prepare/build/deploy/finalize script checks passed"
