#!/bin/sh
set -eu

keep_registry=${UPRAVA_KEEP_REGISTRY_IMAGES:-3}
keep_local=${UPRAVA_KEEP_LOCAL_IMAGES:-1}

case "$keep_registry:$keep_local" in
    *[!0-9:]*|:*)
        echo "Image retention values must be non-negative integers" >&2
        exit 1
        ;;
esac

prune_repository() {
    repository=$1
    keep=$2
    ids=$(docker image ls --filter "reference=$repository:*" --format '{{.ID}}' |
        awk '!seen[$0]++')
    index=0
    for image_id in $ids; do
        index=$((index + 1))
        if [ "$index" -le "$keep" ]; then
            continue
        fi
        docker image rm "$image_id" >/dev/null 2>&1 || true
    done
}

for repository in \
    ghcr.io/zaryalabs/uprava-core \
    ghcr.io/zaryalabs/uprava-web \
    ghcr.io/zaryalabs/uprava-node \
    ghcr.io/zaryalabs/uprava-toolhive; do
    prune_repository "$repository" "$keep_registry"
done

for repository in uprava-core uprava-web uprava-node uprava-toolhive; do
    prune_repository "$repository" "$keep_local"
done

echo "Uprava image retention complete"
