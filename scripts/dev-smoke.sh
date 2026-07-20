#!/bin/sh
set -eu

CORE_URL="${CORE_URL:-http://127.0.0.1:8080}"
WEB_URL="${WEB_URL:-http://127.0.0.1:5173}"
TOOLHIVE_URL="${TOOLHIVE_URL:-http://127.0.0.1:18081}"
SMOKE_RETRIES="${SMOKE_RETRIES:-30}"
SMOKE_DELAY_SECONDS="${SMOKE_DELAY_SECONDS:-2}"
SMOKE_WEB_PASSWORD="${SMOKE_WEB_PASSWORD:-uprava-smoke-password}"
SMOKE_COOKIE_JAR="${SMOKE_COOKIE_JAR:-${TMPDIR:-/tmp}/uprava-dev-smoke-cookies-$$}"
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
    echo "$1 is required for dev smoke checks" >&2
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
      -H "x-uprava-csrf: $CSRF_TOKEN" \
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

wait_for_auth_contains() {
  label="$1"
  url="$2"
  expected="$3"
  attempt=1

  while [ "$attempt" -le "$SMOKE_RETRIES" ]; do
    body="$(auth_get "$url" 2>/dev/null || true)"
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

authenticate() {
  status="$(fetch "$CORE_URL/api/v1/auth/status")"
  auth_required="$(printf '%s' "$status" | json_select field auth_required)"
  if [ "$auth_required" != "true" ]; then
    printf '%s ok\n' "dev auth disabled"
    return 0
  fi

  setup_required="$(printf '%s' "$status" | json_select field setup_required)"
  authenticated="$(printf '%s' "$status" | json_select field authenticated)"
  if [ "$setup_required" = "true" ]; then
    response="$(auth_post_json "$CORE_URL/api/v1/auth/setup" "{\"password\":\"$SMOKE_WEB_PASSWORD\"}")"
    CSRF_TOKEN="$(printf '%s' "$response" | json_select field csrf_token)"
    printf '%s ok\n' "dev auth setup"
    return 0
  fi
  if [ "$authenticated" != "true" ]; then
    response="$(auth_post_json "$CORE_URL/api/v1/auth/login" "{\"password\":\"$SMOKE_WEB_PASSWORD\"}")"
    CSRF_TOKEN="$(printf '%s' "$response" | json_select field csrf_token)"
    printf '%s ok\n' "dev auth login"
    return 0
  fi

  echo "dev auth already authenticated but CSRF token is unavailable" >&2
  return 1
}

require_command curl
require_command grep
require_command node

wait_for_contains "core health" "$CORE_URL/api/v1/health" '"status":"ok"'
wait_for_contains "web entrypoint" "$WEB_URL/" '<title>Uprava</title>'
wait_for_contains "toolhive bridge" "$TOOLHIVE_URL/api/v1/version" '0.40.0'
authenticate
wait_for_auth_contains "core inventory" "$CORE_URL/api/v1/inventory" '"nodes"'

echo "dev smoke passed"
