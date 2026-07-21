#!/bin/sh
set -eu

install_dir=${INSTALL_DIR:-/opt/apps/uprava}
keep=${UPRAVA_KEEP_RELEASES:-3}
releases_dir="$install_dir/builds/releases"
predeploy_backups_dir="$install_dir/backups/pre-deploy"

case "$keep" in
    ''|*[!0-9]*)
        echo "UPRAVA_KEEP_RELEASES must be a non-negative integer" >&2
        exit 1
        ;;
esac

test -d "$releases_dir" || exit 0
active_release=$(sed -n 's/^UPRAVA_RELEASE_ID=//p' "$install_dir/.env.release" 2>/dev/null | head -n 1)
previous_release=$(sed -n 's/^UPRAVA_RELEASE_ID=//p' "$install_dir/.env.previous" 2>/dev/null | head -n 1)
index=0
find "$releases_dir" -maxdepth 1 -type f -name '*.env.release' -exec ls -1t {} + |
    while read -r manifest_path; do
        manifest_name=${manifest_path##*/}
        release=${manifest_name%.env.release}
        index=$((index + 1))
        if [ "$release" = "$active_release" ] || [ "$release" = "$previous_release" ] || [ "$index" -le "$keep" ]; then
            continue
        fi
        case "$release" in
            ''|[!A-Za-z0-9]*|*[!A-Za-z0-9._-]*)
                echo "Refusing unsafe release name: $release" >&2
                exit 1
                ;;
        esac
        rm -f "$releases_dir/$manifest_name"
        rm -rf "$releases_dir/$release"
        rm -rf "$predeploy_backups_dir/$release"
    done

echo "Uprava release retention complete"
