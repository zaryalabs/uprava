#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

CORE_PORT="${CODEX_SMOKE_CORE_PORT:-18080}"
WEB_PORT="${CODEX_SMOKE_WEB_PORT:-15173}"
CORE_URL="${CODEX_SMOKE_CORE_URL:-http://127.0.0.1:$CORE_PORT}"
WEB_URL="${CODEX_SMOKE_WEB_URL:-http://127.0.0.1:$WEB_PORT}"
STATE_DIR="${CODEX_SMOKE_STATE_DIR:-/private/tmp/cortex-codex-provider-smoke-$$}"
WORKSPACE_PATH="${CODEX_SMOKE_WORKSPACE_PATH:-$STATE_DIR/workspace}"
NODE_STATE_PATH="${CODEX_SMOKE_NODE_STATE_PATH:-$STATE_DIR/node.json}"
CORE_DATABASE_URL="${CODEX_SMOKE_DATABASE_URL:-sqlite://$STATE_DIR/core.sqlite}"
EXPECTED_NODE="${CODEX_SMOKE_NODE_NAME:-Host Codex Node}"
SESSION_TITLE="${CODEX_SMOKE_SESSION_TITLE:-Codex provider smoke}"
TURN_CONTENT="${CODEX_SMOKE_TURN_CONTENT:-Reply exactly CORTEX_CODEX_SMOKE_OK. Do not modify files.}"
EXPECTED_ASSISTANT_CONTENT="${CODEX_SMOKE_EXPECTED_ASSISTANT_CONTENT:-CORTEX_CODEX_SMOKE_OK}"
CODEX_TIMEOUT_SECONDS="${CODEX_SMOKE_CODEX_TIMEOUT_SECONDS:-180}"
SMOKE_RETRIES="${SMOKE_RETRIES:-60}"
SMOKE_DELAY_SECONDS="${SMOKE_DELAY_SECONDS:-1}"
CODEX_BINARY="${CORTEX_CODEX_BINARY:-${CODEX_SMOKE_CODEX_BINARY:-}}"
PIDS=""

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

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required for Codex provider smoke checks" >&2
    exit 1
  fi
}

fetch() {
  curl -fsS --noproxy 127.0.0.1,localhost "$1"
}

print_logs() {
  for log in "$STATE_DIR/core.log" "$STATE_DIR/node.log" "$STATE_DIR/web.log"; do
    if [ -f "$log" ]; then
      printf '\n%s\n' "==> $log" >&2
      tail -n 120 "$log" >&2 || true
    fi
  done
}

cleanup() {
  for pid in $PIDS; do
    kill "$pid" >/dev/null 2>&1 || true
  done
  for pid in $PIDS; do
    wait "$pid" >/dev/null 2>&1 || true
  done
}

fail() {
  echo "$1" >&2
  print_logs
  exit 1
}

trap cleanup EXIT
trap 'cleanup; exit 130' INT TERM

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

  if [ -n "${body:-}" ]; then
    printf '%s\n' "$body" >&2
  fi
  fail "$label failed: did not find '$expected' at $url"
}

require_command cargo
require_command curl
require_command git
require_command grep
require_command node
require_command npm

if [ -z "$CODEX_BINARY" ]; then
  CODEX_BINARY="$(command -v codex || true)"
fi
if [ -z "$CODEX_BINARY" ] || ! command -v "$CODEX_BINARY" >/dev/null 2>&1; then
  fail "codex binary is not available; set CORTEX_CODEX_BINARY or CODEX_SMOKE_CODEX_BINARY"
fi

mkdir -p "$STATE_DIR" "$WORKSPACE_PATH"
if [ ! -d "$WORKSPACE_PATH/.git" ]; then
  git -C "$WORKSPACE_PATH" init >/dev/null 2>&1 ||
    fail "failed to initialize disposable git workspace at $WORKSPACE_PATH"
fi

(
  cd "$ROOT_DIR"
  CORTEX_CORE_BIND="127.0.0.1:$CORE_PORT" \
    CORTEX_DATABASE_URL="$CORE_DATABASE_URL" \
    CORTEX_DEPLOYMENT_PROFILE="local_trusted" \
    CORTEX_AUTO_APPROVE_ENROLLMENTS="true" \
    CORTEX_ALLOWED_ORIGINS="$WEB_URL,http://localhost:$WEB_PORT" \
    RUST_LOG="info,cortex_server=debug" \
    cargo run -p cortex-server
) >"$STATE_DIR/core.log" 2>&1 &
PIDS="$PIDS $!"

wait_for_contains "codex core health" "$CORE_URL/api/v1/health" '"status":"ok"'

(
  cd "$ROOT_DIR"
  CORTEX_CORE_URL="$CORE_URL" \
    CORTEX_NODE_DISPLAY_NAME="$EXPECTED_NODE" \
    CORTEX_NODE_HEARTBEAT_SECONDS="1" \
    CORTEX_NODE_STATE_PATH="$NODE_STATE_PATH" \
    CORTEX_NODE_WORKSPACES="$WORKSPACE_PATH" \
    CORTEX_CODEX_BINARY="$CODEX_BINARY" \
    CORTEX_CODEX_TIMEOUT_SECONDS="$CODEX_TIMEOUT_SECONDS" \
    RUST_LOG="info,cortex_node=debug" \
    cargo run -p cortex-node
) >"$STATE_DIR/node.log" 2>&1 &
PIDS="$PIDS $!"

(
  cd "$ROOT_DIR/apps/web"
  VITE_CORTEX_API_BASE="$CORE_URL/api/v1" \
    npm exec vite -- --host 127.0.0.1 --port "$WEB_PORT" --strictPort
) >"$STATE_DIR/web.log" 2>&1 &
PIDS="$PIDS $!"

wait_for_contains "codex web entrypoint" "$WEB_URL/" '<title>Cortex</title>'
wait_for_contains "codex node inventory" "$CORE_URL/api/v1/inventory" "$EXPECTED_NODE"
wait_for_contains "codex provider capability" "$CORE_URL/api/v1/inventory" '"provider.codex"'

(
  cd "$ROOT_DIR"
  CORTEX_E2E_REAL_API=1 \
    CORTEX_E2E_CORE_URL="$CORE_URL" \
    CORTEX_E2E_EXPECTED_NODE="$EXPECTED_NODE" \
    CORTEX_E2E_WORKSPACE_PATH="$WORKSPACE_PATH" \
    CORTEX_E2E_PROVIDER="codex" \
    CORTEX_E2E_SESSION_TITLE="$SESSION_TITLE" \
    CORTEX_E2E_TURN_CONTENT="$TURN_CONTENT" \
    CORTEX_E2E_EXPECTED_ASSISTANT_CONTENT="$EXPECTED_ASSISTANT_CONTENT" \
    CORTEX_E2E_TURN_TIMEOUT_MS="$((CODEX_TIMEOUT_SECONDS * 1000))" \
    CORTEX_E2E_TEST_TIMEOUT_MS="$(((CODEX_TIMEOUT_SECONDS + 30) * 1000))" \
    PLAYWRIGHT_BASE_URL="$WEB_URL" \
    make web-e2e
) || fail "codex provider Playwright E2E failed"

echo "codex provider smoke passed"
