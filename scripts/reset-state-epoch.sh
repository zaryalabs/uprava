#!/bin/sh
set -eu

epoch=${1:?state epoch is required}
core_database=${2:?Core database path is required}
node_database=${3:?Node database path is required}
core_marker=${4:?Core epoch marker path is required}
node_marker=${5:?Node epoch marker path is required}

case "$epoch" in
    ''|*[!A-Za-z0-9._-]*)
        echo "Unsafe state epoch: $epoch" >&2
        exit 1
        ;;
esac

read_marker() {
    if [ -f "$1" ]; then
        sed -n '1p' "$1"
    fi
}

if [ "$(read_marker "$core_marker")" = "$epoch" ] &&
    [ "$(read_marker "$node_marker")" = "$epoch" ]; then
    echo "State epoch $epoch is already active"
    exit 0
fi

for database in "$core_database" "$node_database"; do
    rm -f "$database" "$database-wal" "$database-shm"
done

write_marker() {
    marker=$1
    marker_directory=$(dirname "$marker")
    temporary_marker="$marker.tmp.$$"
    mkdir -p "$marker_directory"
    printf '%s\n' "$epoch" >"$temporary_marker"
    mv "$temporary_marker" "$marker"
}

write_marker "$core_marker"
write_marker "$node_marker"
echo "Reset Core and Node SQLite state for epoch $epoch"
