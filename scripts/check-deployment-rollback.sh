#!/bin/sh
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

grep -q 'UPRAVA_TOOLHIVE_IMAGE:-uprava-toolhive:inactive' "$repo/ops/compose.yaml"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
install="$tmp/install"
releases="$install/builds/releases"
mkdir -p "$tmp/bin" "$releases/old" "$releases/new" "$install/scripts"
cp "$repo/ops/Makefile" "$install/Makefile"
cp "$repo/ops/compose.yaml" "$install/compose.yaml"

write_manifest() {
    release=$1
    sha=$2
    profile=$3
    cat >"$releases/$release.env.release" <<EOF
UPRAVA_RELEASE_ID=$release
UPRAVA_RELEASE_SHA=$sha
UPRAVA_RELEASE_FAMILY=0.2.0
UPRAVA_CORE_STATE_DIR=state/core
UPRAVA_CORE_CONFIG=/etc/uprava/core.env
UPRAVA_NODE_CONFIG=/etc/uprava/node.env
UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server'
UPRAVA_NODE_VERSION=0.2.7+$sha
EOF
    if [ "$profile" = toolhive ]; then
        cat >>"$releases/$release.env.release" <<EOF
UPRAVA_TOOLHIVE_PROFILE=toolhive
UPRAVA_TOOLHIVE_STATE_DIR=state/toolhive
UPRAVA_TOOLHIVE_VERSION=0.40.0
UPRAVA_TOOLHIVE_CONFIG=/etc/uprava/toolhive.env
EOF
    fi
    printf '#!/bin/sh\nexit 0\n' >"$releases/$release/uprava-node"
    chmod 755 "$releases/$release/uprava-node"
}

write_manifest old aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa legacy
write_manifest new bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb toolhive

cat >"$tmp/bin/docker" <<'SH'
#!/bin/sh
printf '%s\n' "$*" >>"${MOCK_DOCKER_LOG:?}"
case " $* " in
    *' exec -T toolhive thv version '*)
        test "${MOCK_HEALTH:-fail}" = pass && printf '%s\n' 'ToolHive 0.40.0'
        ;;
    *' exec -T '*) test "${MOCK_HEALTH:-fail}" = pass ;;
    *) exit 0 ;;
esac
SH
cat >"$tmp/bin/systemctl" <<'SH'
#!/bin/sh
exit 0
SH
cat >"$tmp/bin/sudo" <<'SH'
#!/bin/sh
case "$1" in *=*) exec env "$@" ;; *) exec "$@" ;; esac
SH
cat >"$tmp/bin/curl" <<'SH'
#!/bin/sh
test "${MOCK_HEALTH:-fail}" = pass || exit 1
case "$*" in
    */api/v1/version*) printf '%s\n' '{"release_id":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}' ;;
    *) printf '%s\n' ok ;;
esac
SH
chmod 755 "$tmp/bin/"*

ln -s "$releases/old.env.release" "$install/.env.release"
ln -s "$releases/old" "$install/current"
make -C "$install" --no-print-directory remember-current SUDO= RELEASES_DIR="$releases"
ln -sfn "$releases/new.env.release" "$install/.env.release"
ln -sfn "$releases/new" "$install/current"

if PATH="$tmp/bin:$PATH" MOCK_DOCKER_LOG="$tmp/docker.log" SUDO="$tmp/bin/sudo" MAKE=make INSTALL_DIR="$install" \
    RELEASE_MANIFEST="$releases/new.env.release" UPRAVA_ROOT_PHASE=1 UPRAVA_DOMAIN=example.test \
    FINALIZE_RETRIES=1 bash "$repo/ci/finalize.sh" >"$tmp/finalize.log" 2>&1; then
    echo "Finalize unexpectedly accepted an unhealthy candidate" >&2
    exit 1
fi

test "$(readlink "$install/.env.release")" = "$releases/old.env.release"
test "$(readlink "$install/current")" = "$releases/old"
grep -q 'automatic rollback completed' "$tmp/finalize.log"
rollback_compose=$(tail -n 1 "$tmp/docker.log")
case "$rollback_compose" in *' compose '*' up -d --remove-orphans') ;; *) echo "Rollback did not reapply Compose" >&2; exit 1 ;; esac
case "$rollback_compose" in *'--profile toolhive'*) echo "Legacy rollback kept the ToolHive profile" >&2; exit 1 ;; esac

ln -sfn "$releases/new.env.release" "$install/.env.release"
ln -sfn "$releases/new" "$install/current"
cat >"$install/scripts/prune-uprava-releases.sh" <<'SH'
#!/bin/sh
exit 1
SH
cat >"$install/scripts/prune-uprava-images.sh" <<'SH'
#!/bin/sh
exit 0
SH
chmod 755 "$install/scripts/"*
if PATH="$tmp/bin:$PATH" MOCK_HEALTH=pass MOCK_DOCKER_LOG="$tmp/docker.log" SUDO="$tmp/bin/sudo" MAKE=make INSTALL_DIR="$install" \
    RELEASE_MANIFEST="$releases/new.env.release" UPRAVA_ROOT_PHASE=1 UPRAVA_DOMAIN=example.test \
    FINALIZE_RETRIES=1 bash "$repo/ci/finalize.sh" >"$tmp/retention.log" 2>&1; then
    echo "Finalize unexpectedly accepted failed retention" >&2
    exit 1
fi

test "$(readlink "$install/.env.release")" = "$releases/new.env.release"
test "$(readlink "$install/current")" = "$releases/new"
! grep -q 'automatic rollback completed' "$tmp/retention.log"

rm -f "$install/.env.previous" "$install/previous"
ln -sfn "$releases/new.env.release" "$install/.env.release"
ln -sfn "$releases/new" "$install/current"
if PATH="$tmp/bin:$PATH" MOCK_DOCKER_LOG="$tmp/docker.log" SUDO="$tmp/bin/sudo" MAKE=make INSTALL_DIR="$install" \
    RELEASE_MANIFEST="$releases/new.env.release" UPRAVA_ROOT_PHASE=1 UPRAVA_DOMAIN=example.test \
    FINALIZE_RETRIES=1 bash "$repo/ci/finalize.sh" >"$tmp/bootstrap.log" 2>&1; then
    echo "Finalize unexpectedly accepted an unhealthy first release" >&2
    exit 1
fi

test ! -e "$install/.env.release"
test ! -e "$install/current"
grep -q 'deactivating candidate' "$tmp/bootstrap.log"

echo "Failed finalize restores the previous release or deactivates a first candidate"
