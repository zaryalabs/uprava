#!/bin/sh
set -eu

config_dir=${XDG_CONFIG_HOME:?XDG_CONFIG_HOME is required}/toolhive
config_path=$config_dir/config.yaml

install -d -m 700 "$config_dir"
if [ ! -s "$config_path" ]; then
    install -m 600 /usr/local/share/uprava/toolhive-config.yaml "$config_path"
fi

exec "$@"
