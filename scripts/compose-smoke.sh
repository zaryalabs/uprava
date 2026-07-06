#!/bin/sh
set -eu

CORE_URL="${CORE_URL:-http://127.0.0.1:8080}"
WEB_URL="${WEB_URL:-http://127.0.0.1:5173}"
SMOKE_RETRIES="${SMOKE_RETRIES:-30}"
SMOKE_DELAY_SECONDS="${SMOKE_DELAY_SECONDS:-2}"
EXPECTED_NODE="${EXPECTED_NODE:-Compose Node}"
WORKSPACE_PATH="${WORKSPACE_PATH:-/workspace}"
SMOKE_WEB_PASSWORD="${SMOKE_WEB_PASSWORD:-cortex-smoke-password}"
SMOKE_COOKIE_JAR="${SMOKE_COOKIE_JAR:-${TMPDIR:-/tmp}/cortex-compose-smoke-cookies-$$}"
CSRF_TOKEN=""

case ",${NO_PROXY:-}," in
  *,127.0.0.1,*) ;;
  *) NO_PROXY="${NO_PROXY:+$NO_PROXY,}127.0.0.1" ;;
esac
case ",${NO_PROXY:-}," in
  *,localhost,*) ;;
  *) NO_PROXY="${NO_PROXY:+$NO_PROXY,}localhost" ;;
esac
case ",${no_proxy:-}," in
  *,127.0.0.1,*) ;;
  *) no_proxy="${no_proxy:+$no_proxy,}127.0.0.1" ;;
esac
case ",${no_proxy:-}," in
  *,localhost,*) ;;
  *) no_proxy="${no_proxy:+$no_proxy,}localhost" ;;
esac
export NO_PROXY no_proxy

cleanup() {
  rm -f "$SMOKE_COOKIE_JAR"
}
trap cleanup EXIT

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required for compose smoke checks" >&2
    exit 1
  fi
}

fetch() {
  curl -fsS --noproxy 127.0.0.1,localhost "$1"
}

auth_get() {
  curl -fsS --noproxy 127.0.0.1,localhost -b "$SMOKE_COOKIE_JAR" "$1"
}

auth_post_json() {
  url="$1"
  body="$2"
  if [ -n "$CSRF_TOKEN" ]; then
    curl -fsS \
      --noproxy 127.0.0.1,localhost \
      -b "$SMOKE_COOKIE_JAR" \
      -c "$SMOKE_COOKIE_JAR" \
      -H "content-type: application/json" \
      -H "x-cortex-csrf: $CSRF_TOKEN" \
      -X POST \
      --data "$body" \
      "$url"
  else
    curl -fsS \
      --noproxy 127.0.0.1,localhost \
      -b "$SMOKE_COOKIE_JAR" \
      -c "$SMOKE_COOKIE_JAR" \
      -H "content-type: application/json" \
      -X POST \
      --data "$body" \
      "$url"
  fi
}

json_select() {
  selector="$1"
  shift
  node -e '
const selector = process.argv[1];
const args = process.argv.slice(2);
let input = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  input += chunk;
});
process.stdin.on("end", () => {
  try {
    const data = JSON.parse(input);
    let value;
    if (selector === "field") {
      value = data[args[0]];
    } else if (selector === "enrollment_id") {
      const [expectedNode] = args;
      const enrollment = data.find((candidate) =>
        candidate.display_name === expectedNode &&
        candidate.status === "pending_user_approval"
      );
      value = enrollment && enrollment.enrollment_id;
    } else if (selector === "node_presence") {
      const [expectedNode] = args;
      const node = data.nodes.find((candidate) => candidate.display_name === expectedNode);
      value = node && node.presence;
    } else if (selector === "placement_id") {
      const [expectedNode, workspacePath] = args;
      const node = data.nodes.find((candidate) => candidate.display_name === expectedNode);
      if (!node) throw new Error(`node ${expectedNode} not found`);
      const placement = data.placements.find((candidate) =>
        candidate.node_id === node.node_id &&
        candidate.workspace_path === workspacePath &&
        candidate.state === "validated"
      );
      value = placement && placement.project_placement_id;
    } else if (selector === "provider_capability") {
      const [expectedNode] = args;
      const node = data.nodes.find((candidate) => candidate.display_name === expectedNode);
      const capability = node && node.capabilities.find((candidate) =>
        candidate.key === "provider.codex"
      );
      value = capability ? "present" : "";
    } else {
      throw new Error(`unknown selector ${selector}`);
    }
    if (value === undefined || value === null || value === "") {
      throw new Error(`selector ${selector} returned no value`);
    }
    process.stdout.write(String(value));
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
});
' "$selector" "$@"
}

wait_for_contains() {
  label="$1"
  url="$2"
  expected="$3"
  attempt=1

  while [ "$attempt" -le "$SMOKE_RETRIES" ]; do
    body="$(fetch "$url" 2>/dev/null || true)"
    if printf '%s' "$body" | grep -F "$expected" >/dev/null 2>&1; then
      printf '%s ok\n' "$label"
      return 0
    fi
    sleep "$SMOKE_DELAY_SECONDS"
    attempt=$((attempt + 1))
  done

  echo "$label failed: did not find '$expected' at $url" >&2
  if [ -n "${body:-}" ]; then
    printf '%s\n' "$body" >&2
  fi
  return 1
}

wait_for_auth_value() {
  label="$1"
  url="$2"
  selector="$3"
  expected="$4"
  shift 4
  attempt=1

  while [ "$attempt" -le "$SMOKE_RETRIES" ]; do
    body="$(auth_get "$url" 2>/dev/null || true)"
    value="$(
      printf '%s' "$body" |
        json_select "$selector" "$@" 2>/dev/null || true
    )"
    if [ "$value" = "$expected" ]; then
      printf '%s ok\n' "$label"
      return 0
    fi
    sleep "$SMOKE_DELAY_SECONDS"
    attempt=$((attempt + 1))
  done

  echo "$label failed: expected $selector to be '$expected' at $url" >&2
  if [ -n "${body:-}" ]; then
    printf '%s\n' "$body" >&2
  fi
  return 1
}

wait_for_placement_id() {
  attempt=1
  while [ "$attempt" -le "$SMOKE_RETRIES" ]; do
    body="$(auth_get "$CORE_URL/api/v1/inventory" 2>/dev/null || true)"
    placement_id="$(
      printf '%s' "$body" |
        json_select placement_id "$EXPECTED_NODE" "$WORKSPACE_PATH" 2>/dev/null || true
    )"
    if [ -n "$placement_id" ]; then
      printf '%s ok\n' "compose validated placement"
      return 0
    fi
    sleep "$SMOKE_DELAY_SECONDS"
    attempt=$((attempt + 1))
  done

  echo "compose validated placement failed for $EXPECTED_NODE at $WORKSPACE_PATH" >&2
  if [ -n "${body:-}" ]; then
    printf '%s\n' "$body" >&2
  fi
  return 1
}

authenticate() {
  status="$(fetch "$CORE_URL/api/v1/auth/status")"
  auth_required="$(printf '%s' "$status" | json_select field auth_required)"
  if [ "$auth_required" != "true" ]; then
    printf '%s ok\n' "compose auth disabled"
    return 0
  fi

  setup_required="$(printf '%s' "$status" | json_select field setup_required)"
  authenticated="$(printf '%s' "$status" | json_select field authenticated)"
  if [ "$setup_required" = "true" ]; then
    response="$(auth_post_json "$CORE_URL/api/v1/auth/setup" "{\"password\":\"$SMOKE_WEB_PASSWORD\"}")"
    CSRF_TOKEN="$(printf '%s' "$response" | json_select field csrf_token)"
    printf '%s ok\n' "compose auth setup"
    return 0
  fi
  if [ "$authenticated" != "true" ]; then
    response="$(auth_post_json "$CORE_URL/api/v1/auth/login" "{\"password\":\"$SMOKE_WEB_PASSWORD\"}")"
    CSRF_TOKEN="$(printf '%s' "$response" | json_select field csrf_token)"
    printf '%s ok\n' "compose auth login"
    return 0
  fi

  echo "compose auth already authenticated but CSRF token is unavailable" >&2
  return 1
}

approve_pending_enrollment() {
  attempt=1
  while [ "$attempt" -le "$SMOKE_RETRIES" ]; do
    body="$(auth_get "$CORE_URL/api/v1/node-enrollments" 2>/dev/null || true)"
    enrollment_id="$(
      printf '%s' "$body" |
        json_select enrollment_id "$EXPECTED_NODE" 2>/dev/null || true
    )"
    if [ -n "$enrollment_id" ]; then
      auth_post_json "$CORE_URL/api/v1/node-enrollments/$enrollment_id/approve" "{}" >/dev/null
      printf '%s ok\n' "compose node enrollment approved"
      return 0
    fi
    sleep "$SMOKE_DELAY_SECONDS"
    attempt=$((attempt + 1))
  done

  echo "compose node enrollment approval failed for $EXPECTED_NODE" >&2
  if [ -n "${body:-}" ]; then
    printf '%s\n' "$body" >&2
  fi
  return 1
}

require_command curl
require_command grep
require_command node

wait_for_contains "core health" "$CORE_URL/api/v1/health" '"status":"ok"'
wait_for_contains "web entrypoint" "$WEB_URL/" '<title>Cortex</title>'
authenticate
approve_pending_enrollment
wait_for_auth_value \
  "compose node reachable" \
  "$CORE_URL/api/v1/inventory" \
  node_presence \
  reachable \
  "$EXPECTED_NODE"
wait_for_placement_id
wait_for_auth_value \
  "compose codex capability advertised" \
  "$CORE_URL/api/v1/inventory" \
  provider_capability \
  present \
  "$EXPECTED_NODE"

echo "compose smoke passed"
