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
! grep -q '^cargo ' "$calls"
CALLS="$calls" PATH="$tmp/bin:$PATH" MAKE="$tmp/bin/make" CI_MAIN=1 bash "$repo/ci/prepare.sh" >/dev/null
grep -q '^cargo +1.88.0 check' "$calls"

CALLS="$calls" PATH="$tmp/bin:$PATH" MAKE="$tmp/bin/make" CI_ARTIFACT_DIR="$tmp/handoff" \
    RELEASE_MANIFEST="$tmp/release.env" DOCKER_CONFIG="$tmp/docker-config" bash "$repo/ci/build.sh" >/dev/null
test -s "$tmp/handoff/uprava-release.env"

cat >"$tmp/manifest.env" <<'EOF'
UPRAVA_RELEASE_ID=test
UPRAVA_RELEASE_SHA=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
UPRAVA_NODE_VERSION=0.2.3
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server'
EOF
CALLS="$calls" PATH="$tmp/bin:$PATH" MAKE="$tmp/bin/make" SUDO="$tmp/bin/sudo" \
    INSTALL_DIR="$tmp/install" RELEASE_MANIFEST="$tmp/manifest.env" bash "$repo/ci/deploy.sh" >/dev/null
grep -q 'make install-ops' "$calls"
grep -q 'make -C .* deploy RELEASE=test' "$calls"

cat >"$tmp/install/scripts/prune-uprava-releases.sh" <<'SH'
#!/bin/sh
exit 0
SH
cp "$tmp/install/scripts/prune-uprava-releases.sh" "$tmp/install/scripts/prune-uprava-images.sh"
chmod 755 "$tmp/install/scripts/"*
CALLS="$calls" PATH="$tmp/bin:$PATH" SUDO="$tmp/bin/sudo" INSTALL_DIR="$tmp/install" \
    RELEASE_MANIFEST="$tmp/manifest.env" UPRAVA_DOMAIN=example.test FINALIZE_RETRIES=1 \
    bash "$repo/ci/finalize.sh" >/dev/null

echo "Focused prepare/build/deploy/finalize script checks passed"
