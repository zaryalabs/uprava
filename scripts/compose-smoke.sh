#!/bin/sh
set -eu

CORE_URL="${CORE_URL:-http://127.0.0.1:8080}"
WEB_URL="${WEB_URL:-http://127.0.0.1:5173}"
SMOKE_RETRIES="${SMOKE_RETRIES:-30}"
SMOKE_DELAY_SECONDS="${SMOKE_DELAY_SECONDS:-2}"
EXPECTED_NODE="${EXPECTED_NODE:-Compose Node}"
WORKSPACE_PATH="${WORKSPACE_PATH:-/workspace}"
SMOKE_SESSION_TITLE="${SMOKE_SESSION_TITLE:-Compose smoke session}"
SMOKE_TURN_CONTENT="${SMOKE_TURN_CONTENT:-compose smoke}"
SMOKE_APPROVAL_TURN_CONTENT="${SMOKE_APPROVAL_TURN_CONTENT:-/approval compose smoke approval}"
SMOKE_INTERRUPT_TURN_CONTENT="${SMOKE_INTERRUPT_TURN_CONTENT:-/approval compose smoke interrupt}"
SMOKE_RESUMED_TURN_CONTENT="${SMOKE_RESUMED_TURN_CONTENT:-compose smoke after resume}"
SMOKE_SECOND_TURN_CONTENT="${SMOKE_SECOND_TURN_CONTENT:-compose smoke after node restart}"
SMOKE_ERROR_TURN_CONTENT="${SMOKE_ERROR_TURN_CONTENT:-/error compose smoke provider crash}"
SMOKE_LIFECYCLE_CHECK="${SMOKE_LIFECYCLE_CHECK:-1}"
SMOKE_CORE_RESTART_CHECK="${SMOKE_CORE_RESTART_CHECK:-1}"
SMOKE_NODE_RESTART_CHECK="${SMOKE_NODE_RESTART_CHECK:-1}"
SMOKE_PROVIDER_ERROR_CHECK="${SMOKE_PROVIDER_ERROR_CHECK:-1}"
SMOKE_COMPOSE_FILE="${SMOKE_COMPOSE_FILE:-compose.yaml}"

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
    echo "$1 is required for compose smoke checks" >&2
    exit 1
  fi
}

fetch() {
  curl -fsS --noproxy 127.0.0.1,localhost "$1"
}

post_json() {
  curl -fsS \
    --noproxy 127.0.0.1,localhost \
    -H "content-type: application/json" \
    -X POST \
    --data "$2" \
    "$1"
}

expect_post_json_status() {
  label="$1"
  url="$2"
  body="$3"
  expected_status="$4"
  status="$(
    curl -sS \
      --noproxy 127.0.0.1,localhost \
      -o /dev/null \
      -w "%{http_code}" \
      -H "content-type: application/json" \
      -X POST \
      --data "$body" \
      "$url" || printf '000'
  )"
  if [ "$status" = "$expected_status" ]; then
    printf '%s ok\n' "$label"
    return 0
  fi
  echo "$label failed: expected HTTP $expected_status, got $status at $url" >&2
  return 1
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
    if (selector === "placement_id") {
      const [expectedNode, workspacePath] = args;
      const node = data.nodes.find((candidate) => candidate.display_name === expectedNode);
      if (!node) throw new Error(`node ${expectedNode} not found`);
      const placement = data.placements.find((candidate) =>
        candidate.node_id === node.node_id &&
        candidate.workspace_path === workspacePath &&
        candidate.state === "validated"
      );
      if (!placement) throw new Error(`validated placement ${workspacePath} not found`);
      value = placement.project_placement_id;
    } else if (selector === "session_id") {
      value = data.session && data.session.session_thread_id;
    } else if (selector === "runtime_id") {
      value = data.session && data.session.runtime && data.session.runtime.runtime_session_id;
    } else if (selector === "session_state") {
      value = data.session && data.session.state;
    } else if (selector === "node_presence") {
      const [expectedNode] = args;
      const node = data.nodes.find((candidate) => candidate.display_name === expectedNode);
      value = node && node.presence;
    } else if (selector === "runtime_state") {
      value = data.session && data.session.runtime && data.session.runtime.state;
    } else if (selector === "has_assistant_content") {
      const [expectedContent] = args;
      value = Array.isArray(data.messages) &&
        data.messages.some((message) =>
          message.role === "assistant" &&
          typeof message.content === "string" &&
          message.content.includes(expectedContent)
        )
        ? "true"
        : "false";
    } else if (selector === "has_message_content") {
      const [expectedRole, expectedContent] = args;
      value = Array.isArray(data.messages) &&
        data.messages.some((message) =>
          message.role === expectedRole &&
          typeof message.content === "string" &&
          message.content.includes(expectedContent)
        )
        ? "true"
        : "false";
    } else if (selector === "approval_id") {
      const events = Array.isArray(data.events) ? [...data.events].reverse() : [];
      const event = events.find((candidate) =>
        candidate &&
        candidate.kind === "approval.requested" &&
        candidate.payload &&
        typeof candidate.payload.approval_id === "string"
      );
      value = event && event.payload.approval_id;
    } else if (selector === "has_event_kind") {
      const [expectedKind] = args;
      value = Array.isArray(data.events) &&
        data.events.some((event) => event.kind === expectedKind)
        ? "true"
        : "false";
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

json_body() {
  body_kind="$1"
  shift
  node -e '
const bodyKind = process.argv[1];
const args = process.argv.slice(2);
let body;
if (bodyKind === "session") {
  const [placementId, title] = args;
  body = {
    project_placement_id: placementId,
    title,
    provider: "fake",
  };
} else if (bodyKind === "turn") {
  const [content] = args;
  body = { content };
} else {
  throw new Error(`unknown body kind ${bodyKind}`);
}
process.stdout.write(JSON.stringify(body));
' "$body_kind" "$@"
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

wait_for_placement_id() {
  attempt=1

  while [ "$attempt" -le "$SMOKE_RETRIES" ]; do
    body="$(fetch "$CORE_URL/api/v1/inventory" 2>/dev/null || true)"
    placement_id="$(
      printf '%s' "$body" |
        json_select placement_id "$EXPECTED_NODE" "$WORKSPACE_PATH" 2>/dev/null || true
    )"
    if [ -n "$placement_id" ]; then
      printf '%s ok\n' "compose validated placement" >&2
      printf '%s\n' "$placement_id"
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

wait_for_json_value() {
  label="$1"
  url="$2"
  selector="$3"
  expected="$4"
  shift 4
  attempt=1

  while [ "$attempt" -le "$SMOKE_RETRIES" ]; do
    body="$(fetch "$url" 2>/dev/null || true)"
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

require_command curl
require_command grep
require_command node
if {
  [ "$SMOKE_CORE_RESTART_CHECK" = "1" ] || [ "$SMOKE_NODE_RESTART_CHECK" = "1" ];
} && [ "${SMOKE_SKIP_COMPOSE_UP:-0}" != "1" ]; then
  require_command docker
fi

wait_for_contains "core health" "$CORE_URL/api/v1/health" '"status":"ok"'
wait_for_contains "web entrypoint" "$WEB_URL/" '<title>Cortex</title>'
wait_for_contains "compose node inventory" "$CORE_URL/api/v1/inventory" "$EXPECTED_NODE"

placement_id="$(wait_for_placement_id)"
session_body="$(json_body session "$placement_id" "$SMOKE_SESSION_TITLE")"
session_response="$(post_json "$CORE_URL/api/v1/sessions" "$session_body")"
session_id="$(printf '%s' "$session_response" | json_select session_id)"
runtime_id="$(printf '%s' "$session_response" | json_select runtime_id)"
session_url="$CORE_URL/api/v1/sessions/$session_id"
runtime_url="$CORE_URL/api/v1/runtime-sessions/$runtime_id"

wait_for_json_value "compose runtime start" "$session_url" runtime_state ready

turn_body="$(json_body turn "$SMOKE_TURN_CONTENT")"
post_json "$CORE_URL/api/v1/sessions/$session_id/turns" "$turn_body" >/dev/null
wait_for_json_value \
  "compose fake provider turn" \
  "$session_url" \
  has_assistant_content \
  true \
  "Fake provider accepted: $SMOKE_TURN_CONTENT"
wait_for_json_value "compose runtime ready after turn" "$session_url" runtime_state ready

if [ "$SMOKE_LIFECYCLE_CHECK" = "1" ]; then
  post_json "$CORE_URL/api/v1/sessions/$session_id/detach" "{}" >/dev/null
  wait_for_json_value "compose session detached" "$session_url" session_state detached
  detached_turn_body="$(json_body turn "detached compose smoke should fail")"
  expect_post_json_status \
    "compose detached session rejects turn" \
    "$CORE_URL/api/v1/sessions/$session_id/turns" \
    "$detached_turn_body" \
    400

  post_json "$CORE_URL/api/v1/sessions/$session_id/attach" "{}" >/dev/null
  wait_for_json_value "compose session attached" "$session_url" session_state active

  approval_turn_body="$(json_body turn "$SMOKE_APPROVAL_TURN_CONTENT")"
  post_json "$CORE_URL/api/v1/sessions/$session_id/turns" "$approval_turn_body" >/dev/null
  wait_for_json_value "compose approval blocks runtime" "$session_url" runtime_state blocked
  approval_id="$(fetch "$session_url" | json_select approval_id)"
  approval_body='{"approved":true,"message":"compose smoke approval accepted"}'
  post_json \
    "$CORE_URL/api/v1/sessions/$session_id/approvals/$approval_id/resolve" \
    "$approval_body" >/dev/null
  wait_for_json_value "compose approval resolved event" "$session_url" has_event_kind true approval.resolved
  wait_for_json_value "compose runtime ready after approval" "$session_url" runtime_state ready

  interrupt_turn_body="$(json_body turn "$SMOKE_INTERRUPT_TURN_CONTENT")"
  post_json "$CORE_URL/api/v1/sessions/$session_id/turns" "$interrupt_turn_body" >/dev/null
  wait_for_json_value "compose interrupt setup blocks runtime" "$session_url" runtime_state blocked
  post_json "$runtime_url/interrupt" "{}" >/dev/null
  wait_for_json_value "compose interrupt event persisted" "$session_url" has_event_kind true turn.interrupted
  wait_for_json_value "compose runtime ready after interrupt" "$session_url" runtime_state ready

  post_json "$runtime_url/stop" "{}" >/dev/null
  wait_for_json_value "compose runtime stopped" "$session_url" runtime_state stopped
  wait_for_json_value "compose session stopped" "$session_url" session_state stopped
  post_json "$runtime_url/resume" "{}" >/dev/null
  wait_for_json_value "compose runtime resuming event persisted" "$session_url" has_event_kind true runtime.resuming
  wait_for_json_value "compose runtime ready after resume" "$session_url" runtime_state ready
  wait_for_json_value "compose session active after resume" "$session_url" session_state active

  resumed_turn_body="$(json_body turn "$SMOKE_RESUMED_TURN_CONTENT")"
  post_json "$CORE_URL/api/v1/sessions/$session_id/turns" "$resumed_turn_body" >/dev/null
  wait_for_json_value \
    "compose fake provider turn after resume" \
    "$session_url" \
    has_assistant_content \
    true \
    "Fake provider accepted: $SMOKE_RESUMED_TURN_CONTENT"
  wait_for_json_value "compose runtime ready after resumed turn" "$session_url" runtime_state ready
fi

if [ "$SMOKE_CORE_RESTART_CHECK" = "1" ] && [ "${SMOKE_SKIP_COMPOSE_UP:-0}" != "1" ]; then
  docker compose -f "$SMOKE_COMPOSE_FILE" restart core >/dev/null
  wait_for_contains "compose core health after restart" "$CORE_URL/api/v1/health" '"status":"ok"'
  wait_for_contains "compose node inventory after core restart" "$CORE_URL/api/v1/inventory" "$EXPECTED_NODE"
  wait_for_json_value \
    "compose session history after core restart" \
    "$session_url" \
    has_assistant_content \
    true \
    "Fake provider accepted: $SMOKE_TURN_CONTENT"
  wait_for_json_value "compose runtime ready after core restart" "$session_url" runtime_state ready
fi

if [ "$SMOKE_NODE_RESTART_CHECK" = "1" ] && [ "${SMOKE_SKIP_COMPOSE_UP:-0}" != "1" ]; then
  docker compose -f "$SMOKE_COMPOSE_FILE" restart node >/dev/null
  wait_for_json_value \
    "compose node reachable after node restart" \
    "$CORE_URL/api/v1/inventory" \
    node_presence \
    reachable \
    "$EXPECTED_NODE"
  second_turn_body="$(json_body turn "$SMOKE_SECOND_TURN_CONTENT")"
  post_json "$CORE_URL/api/v1/sessions/$session_id/turns" "$second_turn_body" >/dev/null
  wait_for_json_value \
    "compose fake provider turn after node restart" \
    "$session_url" \
    has_assistant_content \
    true \
    "Fake provider accepted: $SMOKE_SECOND_TURN_CONTENT"
  wait_for_json_value "compose runtime ready after node restart turn" "$session_url" runtime_state ready
fi

if [ "$SMOKE_PROVIDER_ERROR_CHECK" = "1" ]; then
  error_turn_body="$(json_body turn "$SMOKE_ERROR_TURN_CONTENT")"
  post_json "$CORE_URL/api/v1/sessions/$session_id/turns" "$error_turn_body" >/dev/null
  error_message="${SMOKE_ERROR_TURN_CONTENT#/error}"
  error_message="${error_message# }"
  if [ "$error_message" = "$SMOKE_ERROR_TURN_CONTENT" ] || [ -z "$error_message" ]; then
    error_message="Fake provider runtime error"
  fi
  wait_for_json_value "compose fake provider runtime error" "$session_url" runtime_state error
  wait_for_json_value \
    "compose runtime error message persisted" \
    "$session_url" \
    has_message_content \
    true \
    runtime \
    "$error_message"
fi

echo "compose smoke passed"
