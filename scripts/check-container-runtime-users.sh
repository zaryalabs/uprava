#!/bin/sh
set -eu

check_image() {
    file=$1
    expected_user=$2

    final_stage=$(awk 'BEGIN { stage=0 } /^FROM[[:space:]]/ { stage++; last=stage } { lines[stage] = lines[stage] $0 "\n" } END { printf "%s", lines[last] }' "$file")
    user=$(printf '%s\n' "$final_stage" | awk '$1 == "USER" { value=$2 } END { print value }')

    if [ -z "$user" ] || [ "$user" = root ] || [ "$user" = 0 ] || [ "$user" = 0:0 ]; then
        echo "$file: final stage must declare a non-root USER" >&2
        exit 1
    fi
    if [ "$user" != "$expected_user" ]; then
        echo "$file: expected final USER $expected_user, got $user" >&2
        exit 1
    fi
    printf '%s: final USER %s\n' "$file" "$user"
}

check_image Dockerfile.core uprava
check_image Dockerfile.node uprava
check_image apps/web/Dockerfile node

grep -Eq '^ENV HOME=/var/lib/uprava([[:space:]]|\\$)' Dockerfile.core
grep -Eq '^ENV HOME=/var/lib/uprava([[:space:]]|\\$)' Dockerfile.node
grep -Eq '^ENV NODE_ENV=production([[:space:]]|\\$)' apps/web/Dockerfile
grep -q '/data' Dockerfile.core
grep -q '/var/lib/uprava-node/0.2.0' Dockerfile.node
