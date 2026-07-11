#!/bin/sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
make_cmd=${MAKE:-make}
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

release_dir="$tmp_dir/builds/releases"
mkdir -p "$release_dir" "$tmp_dir/configuration"
printf 'UPRAVA_DEPLOYMENT_PROFILE=controlled_dev\nUPRAVA_DATABASE_URL=sqlite:///data/core.sqlite\n' >"$tmp_dir/configuration/core.env"
printf 'UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite\n' >"$tmp_dir/configuration/node.env"
cat > "$release_dir/active.env.release" <<'EOF'
UPRAVA_RELEASE_ID=active
UPRAVA_RELEASE_FAMILY=0.2.0
UPRAVA_CORE_STATE_DIR=state/core
UPRAVA_CORE_CONFIG=configuration/core.env
UPRAVA_NODE_CONFIG=configuration/node.env
UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite
EOF
cat > "$release_dir/prior.env.release" <<'EOF'
UPRAVA_RELEASE_ID=prior
UPRAVA_RELEASE_FAMILY=0.2.0
UPRAVA_CORE_STATE_DIR=state/core
UPRAVA_CORE_CONFIG=configuration/core.env
UPRAVA_NODE_CONFIG=configuration/node.env
UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite
EOF
cat > "$release_dir/mismatch.env.release" <<'EOF'
UPRAVA_RELEASE_ID=other
EOF
ln -s "$release_dir/active.env.release" "$tmp_dir/.env.release"

if (cd "$tmp_dir" && "$make_cmd" -f "$repo_dir/ops/Makefile" --no-print-directory \
    rollback RELEASE=active RELEASES_DIR="$release_dir" SUDO= >/dev/null 2>&1); then
    echo "rollback check: same-release refusal failed" >&2
    exit 1
fi

if (cd "$tmp_dir" && "$make_cmd" -f "$repo_dir/ops/Makefile" --no-print-directory \
    rollback RELEASE=missing RELEASES_DIR="$release_dir" SUDO= >/dev/null 2>&1); then
    echo "rollback check: missing-manifest refusal failed" >&2
    exit 1
fi

if (cd "$tmp_dir" && "$make_cmd" -f "$repo_dir/ops/Makefile" --no-print-directory \
    rollback RELEASE=mismatch RELEASES_DIR="$release_dir" SUDO= >/dev/null 2>&1); then
    echo "rollback check: manifest identity mismatch refusal failed" >&2
    exit 1
fi

if (cd "$tmp_dir" && "$make_cmd" -f "$repo_dir/ops/Makefile" --no-print-directory \
    rollback RELEASE=../prior RELEASES_DIR="$release_dir" SUDO= >/dev/null 2>&1); then
    echo "rollback check: unsafe release traversal refusal failed" >&2
    exit 1
fi

(cd "$tmp_dir" && "$make_cmd" -f "$repo_dir/ops/Makefile" --no-print-directory \
    rollback RELEASE=prior RELEASES_DIR="$release_dir" SUDO= \
    NODE_CONFIG_LINK="$tmp_dir/node.env" >/dev/null)

test "$(readlink "$tmp_dir/.env.release")" = "$release_dir/prior.env.release"
test "$(readlink "$tmp_dir/current")" = "$release_dir/prior"
test "$(readlink "$tmp_dir/.env")" = "configuration/core.env"
test "$(readlink "$tmp_dir/node.env")" = "configuration/node.env"
test -d "$tmp_dir/state/core"
echo "Rollback checks passed"
