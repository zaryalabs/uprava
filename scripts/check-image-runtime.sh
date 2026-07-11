#!/bin/sh
set -eu

: "${UPRAVA_CORE_IMAGE:?set UPRAVA_CORE_IMAGE}"
: "${UPRAVA_WEB_IMAGE:?set UPRAVA_WEB_IMAGE}"
: "${UPRAVA_NODE_IMAGE:?set UPRAVA_NODE_IMAGE}"
: "${UPRAVA_RELEASE_SHA:?set UPRAVA_RELEASE_SHA}"

suffix=$$
network="uprava-runtime-$suffix"
core="uprava-runtime-core-$suffix"
web="uprava-runtime-web-$suffix"
node="uprava-runtime-node-$suffix"
core_port=${UPRAVA_RUNTIME_CORE_PORT:-39080}
web_port=${UPRAVA_RUNTIME_WEB_PORT:-39081}

cleanup() {
    docker rm -f "$node" "$web" "$core" >/dev/null 2>&1 || true
    docker network rm "$network" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

docker network create "$network" >/dev/null
docker run -d --name "$core" --network "$network" \
    --read-only --cap-drop ALL --security-opt no-new-privileges \
    --tmpfs /tmp:rw,noexec,nosuid,nodev,size=64m \
    --tmpfs /data:rw,noexec,nosuid,nodev,size=64m,uid=10001,gid=10001 \
    -e UPRAVA_CORE_BIND=0.0.0.0:8080 \
    -e UPRAVA_DATABASE_URL=sqlite:///data/core.sqlite \
    -e UPRAVA_CORE_LOG_FILE=/data/core.log \
    -e UPRAVA_CLIENT_LOG_FILE=/data/client.log \
    -e UPRAVA_DEPLOYMENT_PROFILE=controlled_dev \
    -e UPRAVA_ALLOWED_ORIGINS="http://127.0.0.1:$web_port" \
    -e UPRAVA_COOKIE_SECURE=false \
    -p "127.0.0.1:$core_port:8080" "$UPRAVA_CORE_IMAGE" >/dev/null

docker run -d --name "$web" --network "$network" \
    --read-only --cap-drop ALL --security-opt no-new-privileges \
    --tmpfs /tmp:rw,noexec,nosuid,nodev,size=16m \
    --tmpfs /var/cache/nginx:rw,noexec,nosuid,nodev,size=16m,uid=101,gid=101 \
    --tmpfs /var/run:rw,noexec,nosuid,nodev,size=1m,uid=101,gid=101 \
    -p "127.0.0.1:$web_port:8080" "$UPRAVA_WEB_IMAGE" >/dev/null

attempt=0
until curl -fsS "http://127.0.0.1:$core_port/api/v1/health" >/dev/null; do
    attempt=$((attempt + 1))
    test "$attempt" -lt 30 || { docker logs "$core"; exit 1; }
    sleep 1
done
curl -fsS "http://127.0.0.1:$web_port/health" | grep -qx ok
version=$(curl -fsS "http://127.0.0.1:$core_port/api/v1/version")
printf '%s' "$version" | grep -q "\"release_id\":\"$UPRAVA_RELEASE_SHA\""
curl -fsS "http://127.0.0.1:$core_port/api/v1/metrics" | grep -q '^uprava_core_requests_total '

docker run -d --name "$node" --network "$network" \
    --read-only --cap-drop ALL --security-opt no-new-privileges \
    --tmpfs /tmp:rw,noexec,nosuid,nodev,size=32m \
    --tmpfs /var/lib/uprava-node:rw,noexec,nosuid,nodev,size=64m,uid=10001,gid=10001 \
    --tmpfs /workspaces:rw,nosuid,nodev,size=64m,uid=10001,gid=10001 \
    -e UPRAVA_CORE_URL="http://$core:8080" \
    -e UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/0.2.0/node.sqlite \
    -e UPRAVA_NODE_WORKSPACES=/workspaces \
    -e UPRAVA_NODE_LOG_FILE=/var/lib/uprava-node/node.log \
    "$UPRAVA_NODE_IMAGE" >/dev/null

attempt=0
until [ "$(docker inspect -f '{{.State.Status}}' "$node")" = running ] \
    && docker exec "$node" sh -c 'test -s /var/lib/uprava-node/node.log && test -s /var/lib/uprava-node/0.2.0/node.sqlite'; do
    attempt=$((attempt + 1))
    status=$(docker inspect -f '{{.State.Status}}' "$node")
    if [ "$status" = exited ] || [ "$attempt" -ge 30 ]; then
        docker logs "$node"
        exit 1
    fi
    sleep 1
done

for container in "$core" "$web" "$node"; do
    test "$(docker inspect -f '{{.HostConfig.ReadonlyRootfs}}' "$container")" = true
    test "$(docker inspect -f '{{.State.Status}}' "$container")" = running
    test "$(docker inspect -f '{{json .HostConfig.CapDrop}}' "$container")" = '["ALL"]'
done
test "$(docker inspect -f '{{.Config.User}}' "$core")" = uprava
test "$(docker inspect -f '{{.Config.User}}' "$web")" = 101
test "$(docker inspect -f '{{.Config.User}}' "$node")" = uprava

echo "Production image runtime check passed for $UPRAVA_RELEASE_SHA"
