# V01 Local Development Runbook

Status: `draft`

This runbook covers the current `0.1` scaffold: Rust Core Backend, Rust Node
Daemon, Vite Web Control Panel, SQLite persistence, the fake-provider path and
the first minimal Codex provider adapter.

## Prerequisites

- Rust toolchain with `cargo`, `rustfmt` and `clippy`.
- Node.js and npm for `apps/web`.
- Docker Compose for the all-in-compose profile.
- `curl`, `grep` and `node` for `make compose-smoke`.

## Local Processes

```sh
make init
make core-r
make node-r
make web-r
```

Default ports and paths:

- Core API: `http://127.0.0.1:8080/api/v1`
- Web UI: `http://127.0.0.1:5173`
- Core SQLite: `.local/state/core.sqlite`
- Node local state: `~/.local/share/cortex-node/node.json`
- Core process log: `.local/logs/core.log`
- Node process log: `.local/logs/node.log`
- Browser client log accepted by Core: `.local/logs/client.log`
- Runtime idle expiry: `CORTEX_RUNTIME_EXPIRY_SECONDS`, default `86400`
- Browser CORS origins: `CORTEX_ALLOWED_ORIGINS`, default
  `http://127.0.0.1:5173,http://localhost:5173`
- Web auth mode: `CORTEX_WEB_AUTH`, default `auto`
- Web session TTL: `CORTEX_WEB_SESSION_TTL_SECONDS`, default `86400`
- Secure cookie flag: `CORTEX_COOKIE_SECURE`, default `false` for local HTTP

Core creates and migrates the SQLite schema on startup. Migration coverage
includes a clean empty database and the previous dev `nodes` table shape without
`credential_hash`. For local-only recovery, stop Core/Node first, copy
`.local/state/core.sqlite` and `~/.local/share/cortex-node/node.json` aside if
you need evidence, then delete the broken local state or use `make
compose-reset` for the Compose volume.

## Security Profiles

`local_trusted` remains the default V01 development profile. It keeps browser
auth disabled, shows the trusted-local warning banner in Web and is intended
only for loopback or otherwise trusted development machines.

`controlled_dev` enables the first security baseline when
`CORTEX_WEB_AUTH=auto`:

- Web shows first-run local password setup, then requires login.
- Core issues an `HttpOnly`, `SameSite=Lax` session cookie and a CSRF cookie.
- Browser mutations must send `x-cortex-csrf`; the Web client does this from
  the CSRF cookie.
- Core checks configured browser origins and records security audit events for
  setup/login/logout, rejected auth, CSRF failures, enrollment, revoke and
  rotate.
- Node heartbeat and control-channel auth use bearer credentials. Core stores
  only credential hashes and verifies them with constant-time comparison.

For HTTPS or a TLS-terminating proxy, set `CORTEX_COOKIE_SECURE=true`. For
local HTTP development, leave it disabled or the browser will not return the
session cookie. `CORTEX_WEB_AUTH=local` forces auth in any profile;
`CORTEX_WEB_AUTH=disabled` disables it explicitly.

## Docker Compose

```sh
make compose-smoke
make compose-logs
make compose-down
```

Use `make compose-up` instead when you want the services attached in the
foreground for interactive debugging. `make compose-smoke` starts or rebuilds
the Compose profile in detached mode before running checks. Set
`SMOKE_SKIP_COMPOSE_UP=1` to probe an already running non-default profile.

Reset the local Compose state intentionally:

```sh
make compose-reset
```

The Compose profile starts Core, Web and a synthetic Node Daemon that heartbeats
to Core. Host ports are bound to `127.0.0.1`. Compose enables
`CORTEX_AUTO_APPROVE_ENROLLMENTS=true` for the synthetic Node only. The
fake-provider path remains the deterministic default, so Compose does not
require Codex.

Core rejects browser CORS origins outside `CORTEX_ALLOWED_ORIGINS`; the default
allows the local Vite Web UI on `127.0.0.1` and `localhost`. For a controlled
development host or forwarded port, set `CORTEX_ALLOWED_ORIGINS` to the exact
comma-separated browser origins that should reach Core. Wildcard origins are
rejected in this trusted-development profile.

`make compose-smoke` starts the profile and then runs
`scripts/compose-smoke.sh`. The script bypasses localhost HTTP proxies and
checks:

- Core health at `http://127.0.0.1:8080/api/v1/health`;
- Web entrypoint at `http://127.0.0.1:5173`;
- Core inventory contains the synthetic `Compose Node`;
- the synthetic Node has a validated `/workspace` placement;
- Core can create a fake-provider session on that placement;
- Node receives the start command, returns the runtime to `ready`, processes a
  turn and persists a fake-provider assistant message;
- Core can restart and reload the same session history and ready runtime state
  from SQLite while the Compose Node remains running;
- the Compose Node can restart, reconnect, and process another turn for the
  same persisted session;
- the fake provider can emit a deterministic runtime error, Core persists the
  runtime message, and the session runtime state becomes `error`.

Override `CORE_URL`, `WEB_URL`, `EXPECTED_NODE`, `WORKSPACE_PATH`,
`SMOKE_SESSION_TITLE`, `SMOKE_TURN_CONTENT`, `SMOKE_SECOND_TURN_CONTENT`,
`SMOKE_ERROR_TURN_CONTENT`, `SMOKE_RETRIES`, `SMOKE_DELAY_SECONDS`,
`SMOKE_COMPOSE_FILE`, `SMOKE_CORE_RESTART_CHECK`, `SMOKE_NODE_RESTART_CHECK` or
`SMOKE_PROVIDER_ERROR_CHECK` when running the same smoke check against a
non-default local profile. Set `SMOKE_SKIP_COMPOSE_UP=1` when those endpoints
are already running and should not be started by the Make target. Set
`SMOKE_CORE_RESTART_CHECK=0`, `SMOKE_NODE_RESTART_CHECK=0` or
`SMOKE_PROVIDER_ERROR_CHECK=0` when the script must not restart the target Core
or Node service or must leave the smoke session in a ready runtime state.

## Node Enrollment

For a host-running Node, leave `CORTEX_AUTO_APPROVE_ENROLLMENTS` disabled on
Core. Start the Node:

```sh
make node-r
```

The Node writes local development state to
`~/.local/share/cortex-node/node.json` by default and logs the short-lived
`enrollment_id`. That state includes a stable `daemon_installation_id` used for
local diagnostics; the pairing code stays in local Node state for the claim
request and is not logged. Approve the enrollment through Core:

```sh
curl -X POST http://127.0.0.1:8080/api/v1/node-enrollments/{enrollment_id}/approve
```

The Node retries the claim, stores the returned development credential in its
local state file with private file permissions where the OS supports them, and
then heartbeats with that credential in the `Authorization: Bearer` header.
Heartbeats include
bounded Node diagnostics with the daemon installation id, local event outbox
count and command cache count so the Core node detail can distinguish local
state files during troubleshooting. Revoking a node clears the Core-side
credential hash and rejects future heartbeats from that node. Rotating a node
credential returns a new credential once; update the Node state before
restarting that daemon or the old credential will be rejected.

## Control Channel

After enrollment, Core asks the Node to open `/api/v1/node/control` when there
are pending commands. Node connects outbound over WebSocket with its development
credential in the authorization header, sends a `control.hello` frame, receives
`control.hello_ack` after protocol compatibility is checked, then receives
`command.dispatch` frames and returns `command.ack`, `event_batch` and
`command.result` frames. If either side receives a control frame with an
unsupported protocol version, it replies with `control.error` using
`control.protocol_incompatible` and does not execute that command batch.

Core records node-routed commands before dispatch. The command envelope remains
stored as JSON for replay, and the command table also keeps queryable actor,
correlation, source refs, cause refs, payload and dedupe-key fields for
attribution, inspection and future idempotency checks.
Accepted events follow the same shape: Core stores the full event JSON and
queryable actor, scope, correlation, source/evidence/cause/result refs and
payload fields for ordered projections and inspection.

The current fake-provider path executes through the Node-side Provider Adapter
boundary for `StartRuntime`, `ResumeRuntime`, `SendTurn`, `InterruptRuntime`
and `StopRuntime`. Fake `SendTurn` events mark the runtime running, emit turn
and provider message events, then return the runtime to ready. Core persists
accepted Node events and rebuilds assistant messages from
`provider.message.completed`. Core also stores a durable `turns` row for every
accepted user turn and advances that row from `turn.started`,
`approval.requested`, `turn.completed`, `turn.interrupted` and runtime error
events. Approval request and resolution events are also projected into the
durable `approvals` table. Node keeps generated events in a local outbox until
Core acknowledges their event ids, so reconnects can replay unaccepted
fake-provider events without regenerating command output.
Node command dedupe stores the terminal `command.result` status: normal event
batches complete the command, while provider/runtime error batches fail it and
replay the same failed status for duplicate command delivery.
The outbox is bounded; if retention is exceeded, Node drops the oldest unacked
events and emits a runtime-scoped `runtime.error` with
`node.event_outbox_retention_exceeded` so Core and UI see an explicit degraded
history condition instead of silent loss.

Node also keeps a small local runtime-state projection from its own emitted
events. Heartbeats report `active_runtime_count`, and `control.hello` reports
active runtime ids from that projection. `runtime.ready`, `runtime.running`,
`runtime.blocked`, `runtime.resuming` and interrupted runtimes count as active;
`runtime.stopped`, `runtime.error` and `runtime.expired` do not.

## Logs And Redaction

Use `RUST_LOG=info,cortex_server=debug,cortex_node=debug` for local diagnosis.
Core and Node write the same tracing stream to stderr and append local files by
default:

```sh
tail -f .local/logs/core.log
tail -f .local/logs/node.log
tail -f .local/logs/client.log
```

Override the file locations when running outside the repository root:

```sh
export CORTEX_CORE_LOG_FILE=/tmp/cortex-core.log
export CORTEX_NODE_LOG_FILE=/tmp/cortex-node.log
export CORTEX_CLIENT_LOG_FILE=/tmp/cortex-client.log
```

The Web client installs global `error` and `unhandledrejection` handlers and
logs failed Core API calls and session-stream errors to
`POST /api/v1/client/logs`. Core stores those records as JSONL in
`CORTEX_CLIENT_LOG_FILE`, including browser route, user agent, client timestamp,
source, level, message and bounded diagnostic detail.

Core emits structured logs for enrollment create/approve/claim, heartbeat
acceptance, control-channel connect/disconnect, command record/dispatch/result,
event append, stream gaps and runtime-state changes. These logs use IDs,
states, counts and command `correlation_id` values; they intentionally avoid
command payloads, bearer tokens, node credentials and pairing codes.
HTTP command APIs use `x-correlation-id` when supplied, fall back to
`x-request-id`, and otherwise generate a fresh correlation id before recording
the command. Command-generated events copy that value, and Core backfills it
from `command_id` when accepting older Node event payloads.

The host-running Node logs the short-lived `enrollment_id` during first-run
enrollment so the operator can approve it in Core. The pairing code and
persistent development credential are kept only in the local Node state file
and are not logged. Node local-state debug formatting redacts both `credential`
and `pairing_code`.

Workspace validation is also routed through the control channel. The placement
API creates a pending placement and records a `ValidateWorkspace` command. Node
validates the path on the machine that owns the workspace, emits a
`workspace.validated` event scoped to the placement, and Core projects that
event into the durable placement state and resource badges. Git workspaces also
get lightweight resource badges from `git status --porcelain=v1 --branch`,
including branch, dirty workspace, ahead and behind state when available. Core
adds a computed `same_workspace_active` warning badge when a workspace already
has live session work; the warning stays non-blocking in `0.1`.

Core runs command preflight before recording node-routed work. Validation,
session start, turn send, approval resolution, stop, interrupt and resume reject
revoked or offline nodes. Runtime start, turn send and resume also reject
placements that are not validated or have hard-blocking resource badges. The
runtime start, turn send, approval resolution and resume paths require the node
to advertise the selected `provider.*` capability. Heartbeats replace Core's
normalized `node_capabilities` snapshot, and provider routing reads that
snapshot while the public node API keeps the compatibility capability list.
Runtime-state preflight allows new turns only while the runtime is ready or
running, approval resolution only for a pending approval while the runtime is
blocked, interrupt only while running or blocked, resume only from stopped,
expired, stale, error or interrupted runtimes, and stop only before stopped or
expired. The agent
projection uses the same basic signals when advertising available commands, and
adds `node_stale`, `node_offline`, `node_revoked` or `provider_unavailable`
warnings when heartbeat or capability state requires it.

Placement resource snapshots can be refreshed with
`POST /api/v1/placements/{placement_id}/resource-snapshot/refresh`. Core records
a `RefreshResourceSnapshot` command, Node emits `resource.snapshot.updated` with
the current workspace state and resource badges, and Core projects that event
back onto the placement.

Resource and runtime warnings can be acknowledged through Core with
`POST /api/v1/sessions/{session_thread_id}/warnings/{warning_kind}/acknowledge`.
Core records a session-scoped `coordination.warning_acknowledged` event, stores
the acknowledgement in `warning_acknowledgements`, and removes that warning kind
from the session's active warning projection.

Session attachment is Core-only state. Use
`POST /api/v1/sessions/{session_thread_id}/detach` to mark a session detached
without stopping the runtime, and
`POST /api/v1/sessions/{session_thread_id}/attach` to reattach it. Detached
sessions remain readable and manageable, but Core rejects new turns and approval
responses until they are attached again. Healthy runtime activity does not
reattach a detached session; only the explicit attach endpoint does.

For deterministic local checks, send `/approval <prompt>` to emit an
approval request and block the runtime, or `/error <message>` to emit a runtime
error. Approval requests can be resolved through Core as `ResolveApproval`
commands.

## Codex Provider

The Node can also run a minimal Codex adapter when a session is created with
`provider: "codex"`. Configure it with:

```sh
export CORTEX_CODEX_BINARY=codex
export CORTEX_CODEX_TIMEOUT_SECONDS=120
```

Node advertises `provider.codex` as available only when `CORTEX_CODEX_BINARY`
is either an existing path or resolves through `PATH`; otherwise Core preflight
can reject Codex session start or turn commands with a missing provider
capability instead of waiting for runtime execution to fail.

The adapter stores the runtime provider, workspace path, bounded node-local
transcript and provider resume ref from `StartRuntime` or `RuntimeReady` in
Node local state. When no provider session id is known, `SendTurn` builds a
prompt from the recent local transcript plus the latest user turn, then runs:

```text
codex exec --cd <workspace_path> --json --output-last-message <temp_file> <turn>
```

When a bounded `provider_resume_ref` includes a Codex session id, `SendTurn`
uses the provider-native non-interactive resume path instead and sends only the
latest user turn:

```text
codex exec resume --json --output-last-message <temp_file> <session_id> <turn>
```

Node normalizes the result into `runtime.running`, `turn.started`,
`provider.output.delta`, `provider.message.completed`, `turn.completed` and
`runtime.ready`. JSON stdout events that look like provider approval or
user-input requests are normalized into `approval.requested` plus
`runtime.blocked` without storing raw provider payloads. JSON stdout session
or cursor identifiers are captured as a bounded `provider_resume_ref` and Core
persists that object on the runtime projection. `ResumeRuntime` carries the
persisted provider resume ref back to Node when available so the next turn can
use `codex exec resume`; without that ref, the adapter falls back to the
bounded node-local transcript path. Missing binary, startup failure, timeout,
non-zero exit and empty final output map to `runtime.error` with user-safe
provider codes:
`provider.workspace_missing`, `provider.missing_binary`,
`provider.start_failed`, `provider.start_timeout`, `provider.exec_failed` and
`provider.empty_output`.

This is the accepted V01 Codex protocol after the local CLI spike: an exec-mode
adapter, not the final provider-native persistent runtime. `ResumeRuntime` can
return to `ready` by restoring a persisted provider resume ref or, when no
provider resume ref exists, by using the bounded node-local transcript as
context for future turns. Persistent interactive ownership, live streaming and
real interrupt escalation are post-V01 work.
`StopRuntime` marks the runtime stopped; `InterruptRuntime` currently reports
an explicit unsupported provider error for Codex exec mode.

When the host has an authenticated Codex CLI, run the host Codex smoke:

```sh
make codex-smoke
```

This starts a disposable Core on `127.0.0.1:18080`, Web on
`127.0.0.1:15173`, and a host Node with a temporary git workspace under
`/private/tmp`. It creates a `provider: "codex"` session through Core, sends a
single constrained turn through the Node adapter, and verifies the resulting
session in the Web Control Panel with Playwright. Override
`CODEX_SMOKE_CORE_PORT`, `CODEX_SMOKE_WEB_PORT`, `CODEX_SMOKE_STATE_DIR`,
`CODEX_SMOKE_WORKSPACE_PATH`, `CODEX_SMOKE_CODEX_BINARY`,
`CODEX_SMOKE_TURN_CONTENT`, `CODEX_SMOKE_EXPECTED_ASSISTANT_CONTENT` or
`CODEX_SMOKE_CODEX_TIMEOUT_SECONDS` for a non-default host profile. The script
uses real Codex CLI auth/state and model access; keep it separate from
deterministic `make c` and Compose smoke checks.

Core deduplicates replayed events by `event_id`. A conflicting `seq` is
rejected, and a detected sequence gap marks the session degraded and the
runtime stale with a degraded reason for UI/read-model consumers.

The session artifact tree and agent projection are rebuilt from Core
persistence. The projection includes current turn, pending approvals, active
warnings, recent message refs, available commands and a safe resume context.
Runtime-scoped events update `last_runtime_step_at`; healthy runtime events
such as `runtime.ready` clear degraded runtime/session read-model state. Core
expires ready, running, blocked or stale runtimes after
`CORTEX_RUNTIME_EXPIRY_SECONDS` without runtime activity by recording a
system-authored `runtime.expired` event. Expired runtimes reject new turns but
remain resumable when the provider adapter supports resume.
The session SSE endpoint sends persisted historical events first and then keeps
the connection open for future accepted events through Core's in-process event
bus. If the stream falls behind the bounded bus, Core emits a `cortex.reload`
SSE event so the client can refetch the snapshot.

## Golden Path

1. Start Core and Web, then start a Node Daemon.
2. Open `http://127.0.0.1:5173`.
3. Open the reachable node.
4. Register a workspace placement with an explicit path.
5. Start a fake-provider session, or a Codex session where the Codex CLI is
   available.
6. Send a turn and verify that user, assistant and runtime event blocks appear.
7. Stop or resume the runtime from the session header controls.
8. Reload the browser and verify that inventory, placement, session messages,
   artifact tree and agent projection reload from Core.

## Checks

```sh
make c
make compose-smoke
make web-e2e
make codex-smoke
```

`make compose-smoke` covers the deterministic fake-provider path through
Compose: session start, turn send, detach/attach, detached-turn rejection,
approval resolve, interrupt from a blocked runtime, stop/resume, post-resume
turn send, Core restart, Node restart and provider-error projection. Set
`SMOKE_LIFECYCLE_CHECK=0`, `SMOKE_CORE_RESTART_CHECK=0`,
`SMOKE_NODE_RESTART_CHECK=0` or `SMOKE_PROVIDER_ERROR_CHECK=0` only when
narrowing a local investigation.

`make web-e2e` starts the Vite web server when `PLAYWRIGHT_BASE_URL` is not set.
Set `PLAYWRIGHT_BASE_URL` to run the same checks against an already running
local profile.
Use `PLAYWRIGHT_BASE_URL=http://127.0.0.1:5173 make web-e2e` for automated
browser checks against the Compose Web service. The default E2E run uses
mocked Core snapshots for deterministic UI warning/degraded-state assertions.
To run the real Core/Web/Node fake-provider browser path against the same
profile, first run `make compose-smoke`, then run:

```sh
CORTEX_E2E_REAL_API=1 \
PLAYWRIGHT_BASE_URL=http://127.0.0.1:5173 \
make web-e2e
```

For agent/operator inspection, run the same Compose profile, open
`http://127.0.0.1:5173` with `playwright-cli`, verify the trusted profile
banner, inventory tree, workspace/session flow, warning/degraded states and
inspector actions, then collect `make compose-logs` output if a defect needs
debugging.

`make codex-smoke` is the real-provider counterpart to the fake-provider
Compose path. Run it only where Codex CLI is installed and authenticated.

## Known Limits

- Control-channel event outbox persistence covers Node-generated fake-provider
  and minimal Codex exec-mode events. Codex continuity uses a bounded
  node-local transcript when no provider session id is known, and uses
  `codex exec resume` when a provider session id is available. Full reconnect
  integration coverage and provider resume edge-case repair are still
  incomplete.
- Warning acknowledgements are scoped by session and warning kind; future
  resource-specific acknowledgement expiry is not implemented yet.
- Node enrollment credentials are development credentials only; this remains
  trusted local or controlled-development.
- Host Node workspace summaries are still reported through heartbeat snapshots
  for paths in `CORTEX_NODE_WORKSPACES`; explicit UI-created placement
  validation now runs through a Node `ValidateWorkspace` command.
- The Codex provider adapter is the V01 exec/resume mode with bounded local
  transcript continuity plus provider-native non-interactive resume when a
  provider session id is available. Persistent interactive process ownership,
  streaming output and real interrupt escalation are post-V01 work.
