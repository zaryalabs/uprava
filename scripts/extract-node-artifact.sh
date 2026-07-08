#!/usr/bin/env bash
set -euo pipefail

image="${1:?image ref is required}"
output="${2:?output path is required}"

checksum_file() {
  local path="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
    return
  fi

  printf "sha256sum or shasum is required\n" >&2
  exit 1
}

if ! docker image inspect "$image" >/dev/null 2>&1; then
  docker pull "$image" >/dev/null
fi

output_dir="$(dirname "$output")"
mkdir -p "$output_dir"

tmp_output="$(mktemp "${output}.tmp.XXXXXX")"
container_id="$(docker create "$image")"

cleanup() {
  docker rm -f "$container_id" >/dev/null 2>&1 || true
  rm -f "$tmp_output"
}
trap cleanup EXIT

docker cp "$container_id:/usr/local/bin/uprava-node" "$tmp_output"
chmod 755 "$tmp_output"
node_sha="$(checksum_file "$tmp_output")"
mv "$tmp_output" "$output"
trap - EXIT
docker rm -f "$container_id" >/dev/null 2>&1 || true

printf "%s\n" "$node_sha"
