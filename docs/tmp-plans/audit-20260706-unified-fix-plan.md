# 2026-07-06 Unified Audit Fix Plan

Date: 2026-07-06

Status: `implemented-with-followups`

Purpose: объединить результаты deep audit и локального Clawpatch report
`.clawpatch/reports/20260706T131325-7b77ff.md` в один рабочий план без
дублирующихся задач. Этот документ заменяет два прежних temporary plan:
`deep-audit-20260706-fix-plan.md` и `clawpatch-20260706-fix-plan.md`.

Implementation outcome: the behavior, security, durability, quality-gate,
operator-feedback, healthcheck/logging and release-baseline fixes landed as
the `0.1.6` unified audit hardening baseline. Broad mechanical refactors that
do not change shipped behavior - Core/Node module splitting, generated protocol
contracts, and an async workspace command API - are intentionally deferred to
feature queue item 17.

Verification update, 2026-07-06: the remaining current-blocker implementation
gaps are closed. Node runtime workspace paths are canonicalized through
`UPRAVA_NODE_WORKSPACES` on `StartRuntime`/`ResumeRuntime` and rechecked before
Codex `SendTurn`; duplicate workspace command replay persists and returns typed
result payloads after restart; Core HTTP routing has bounded request body and
timeout layers. Regression coverage was added for the workspace escape,
runtime reauthorization, duplicate replay and oversized-body cases, and
`make c` passes.

## Related Docs

- [`docs/en/architecture.md`](../en/architecture.md) - Core как control plane,
  Node Daemon как data plane и запрет прямой зависимости клиентов от nodes.
- [`docs/en/v01.md`](../en/v01.md) - historical first product cut shipped as
  `0.1.0`, trusted development deployment, session/runtime lifecycle и visible
  error/offline states.
- [`docs/en/versioning.md`](../en/versioning.md) - SemVer rules and current
  `0.1.6` release baseline.
- [`docs/en/releases.md`](../en/releases.md) - shipped implementation slices
  from `0.1.0` through `0.1.6`.
- [`docs/en/tech-stack.md`](../en/tech-stack.md) - Rust/Axum/Tokio/SQLite,
  React/Vite/TypeScript и ожидаемые local quality gates.
- [`docs/en/design/003-distributed-runtime-coordination.md`](../en/design/003-distributed-runtime-coordination.md)
  - idempotent commands, ordered runtime/session events и reconnect semantics.
- [`docs/en/workspace-inspector.md`](../en/workspace-inspector.md) - workspace
  inspector direction and the surface that has now shipped through `0.1.6`.
- [`docs/en/runbooks/v01-local-dev.md`](../en/runbooks/v01-local-dev.md) -
  controlled dev security, node auth, control channel и local `0.1.x` checks.

## Scope

In scope:

- Исправить runtime correctness bugs, которые могут зависать команды, терять
  session events или повторять side effects.
- Закрыть подтвержденные Node security gaps: provider workspace allow-list,
  stale credentials, symlink-safe file writes и bounded command execution.
- Сделать Node local state и event outbox устойчивыми к crash/reconnect
  сценариям.
- Сделать `make c` и связанные quality targets честными: scaffolded проверки
  не должны silently skip или возвращать false success.
- Синхронизировать Rust MSRV contract с `Cargo.lock` и local/CI checks.
- Синхронизировать current `0.1.6` baseline с фактически реализованным
  Workspace Inspector and command runner.
- Исправить Web Control Panel states, где ошибки скрываются как loading/not
  found или теряется пользовательский ввод.
- Уменьшить архитектурное сцепление в Core Backend и Node Daemon после
  behavior fixes.
- Добавить focused regression tests для каждого исправленного класса ошибок.

Out of scope:

- Production RBAC/permission модель.
- Замена Codex provider architecture на app-server или другой runtime.
- Миграция SQLite на Postgres.
- Большой UI redesign.
- Полная генерация всего API SDK, если достаточно узкого protocol/schema gate
  для текущей `0.1.x` baseline.
- Переписывание всего workspace inspector или session UI без необходимости для
  исправления найденных bugs.

## Consolidated Triage

Критический порядок работ:

1. Quality gate honesty, чтобы последующие исправления проверялись надежно.
2. Node trust boundary and local state safety.
3. Command lifecycle, retry semantics and idempotent replay.
4. Session event projection and live update reliability.
5. Web-visible operator feedback and draft preservation.
6. Workspace Inspector release baseline and explicit tool policy.
7. Runtime limits, healthcheck and logging hardening.
8. Schema/protocol contract hygiene.
9. Mechanical module boundary cleanup after behavior is covered by tests.

Deduplication decisions:

- Empty-session SSE appears once in Slice 4.
- Symlink-safe workspace writes and stale credential resurrection appear once in
  Slice 2.
- Duplicate command handling, typed payload replay and ACK-after-crash retry
  appear once in Slice 3.
- Bounded workspace process output appears once in Slice 7.
- Workspace Inspector release-baseline cleanup is separated from low-level Node
  security: the security fixes apply regardless of documentation cleanup.

## Slice 1: Quality Gate And MSRV Honesty

Goal: локальная команда `make c` должна быть надежным handoff gate, а
`rust-version` должен соответствовать locked dependency graph.

Findings:

- `Cargo.lock` resolves dependencies above the declared Rust 1.80 MSRV.
- Quality recipes can pass after earlier check failures.
- Web quality gate silently skips checks when dependencies are missing.
- Rust TOML check can fail on missing optional paths.
- Clawpatch command targets are omitted from `.PHONY`.

Plan:

1. Добавить `set -e` или явные `&&`/failure branches в multi-command recipes:
   `init`, `rust-l`, `rust-tools-install`, `web-l` and similar targets.
2. For `web-fmt`, `web-l`, `web-dl`, `web-t`: if `apps/web/package.json`
   exists and `apps/web/node_modules` is missing, return non-zero with a clear
   `run make init` message.
3. Keep non-fatal skips only for dev-server/e2e targets where operator
   convenience is intended.
4. Make `RUST_TOOL_TOML_FILES` existing-file-only through `$(wildcard ...)`.
5. Add all `claw-*` command targets to `.PHONY`.
6. Make an explicit MSRV decision:
   - if Rust 1.80 stays the contract, pin/regenerate the dependency graph and
     add an MSRV check;
   - if the current dependency graph is the contract, raise
     `[workspace.package].rust-version` and update docs/CI.
7. Add `cargo +<msrv> check --workspace --all-targets --locked` to CI or to a
   documented local gate once CI is formalized.
8. Add Makefile smoke tests or lightweight shell tests for:
   - first subcommand fails, second succeeds, target still fails;
   - scaffolded web dependencies are missing, quality target fails;
   - same-named file cannot mask `claw-review`.

Completion criteria:

- `make c` cannot pass if scaffolded web checks were not actually run.
- `make l`, `web-l` and `rust-l` return non-zero when any required subcommand
  fails.
- MSRV contract and `Cargo.lock` no longer conflict.
- Optional stack parts can still be absent, but scaffolded stack dependencies
  are not hidden.

## Slice 2: Node Trust Boundary And Local State Durability

Goal: Node Daemon не должен выполнять provider/workspace операции вне
разрешенных workspace roots, resurrect stale credentials, corrupt local state
or lose outbox events across crash/reconnect.

Findings:

- Codex runtime workspace path can bypass the Node allow-list.
- Cleared credentials can be resurrected by an old control-channel task.
- Workspace file writes have a symlink race after validation.
- Node local state stores credential, command status, runtime seqs, event
  outbox and provider resume refs in one JSON file.
- Saves use truncate/write without temp file, rename, fsync or lock.
- Main loop and control-channel task hold separate clones and can overwrite
  each other.
- Live events can be sent to Core before updated outbox/seq state is durably
  saved.

Plan:

1. Validate `workspace_path` from `StartRuntime` and `ResumeRuntime` through
   the existing Node workspace allow-list before saving runtime metadata.
2. Store only canonical authorized paths in `runtime_workspace_paths`.
3. Before `CodexProviderAdapter::run_codex_exec`, re-check the stored workspace
   path and emit `runtime.error` if it is no longer allowed.
4. Introduce one authoritative owner for mutable Node local state:
   - preferred: state actor with command messages;
   - acceptable first step: shared `Arc<Mutex<_>>` with a disciplined save API.
5. Replace direct `save()` calls with state-store methods:
   - `record_command_ack`;
   - `record_command_result`;
   - `append_outbox_event`;
   - `ack_outbox_events`;
   - `clear_registration`.
6. Cancel or fence old control-channel tasks before `clear_core_registration`:
   keep a `JoinHandle`, cancellation token or generation/identity guard.
7. Prevent stale identity writes after credential clearing with generation or
   identity checks in the state store.
8. Make state writes atomic:
   - write to a temp file in the same directory;
   - flush the file;
   - rename over the target;
   - fsync parent directory where supported;
   - preserve private file permissions.
9. Persist outbox event and seq before live send, or mark live send
   best-effort with durable replay guaranteed on reconnect.
10. Replace workspace write check-then-open with a symlink-safe helper:
    - open target with no-follow/platform equivalent where available;
    - do not truncate before holding a verified handle;
    - validate post-open metadata;
    - check expected content through the opened handle;
    - reject symlink race swaps.
11. Add tests:
    - allowed temp workspace and disallowed temp workspace;
    - `StartRuntime`/`SendTurn` cannot launch provider outside allowed roots;
    - heartbeat auth rejection while control channel is active cannot restore
      old `node_id`/credential;
    - interrupted write does not corrupt existing local state;
    - live event accepted by Core is not followed by Node seq rollback;
    - symlink race cannot write outside workspace.

Completion criteria:

- Codex provider cannot launch outside `UPRAVA_NODE_WORKSPACES`.
- Clearing node credentials cannot be undone by an older control-channel task.
- Node state writes are atomic at process-crash granularity.
- Event replay after reconnect is deterministic and does not create seq
  rollback conflicts.
- Workspace write path does not follow symlinks even under race.

## Slice 3: Command Lifecycle, Retry Semantics And Idempotency

Goal: команда не должна зависать после transport ACK, терять typed result
payload on retry, or repeat side effects for duplicate delivery.

Findings:

- Core dispatch reselects only `recorded`, `pending_dispatch` and
  `dispatched`.
- Node sends `CommandAck` before executing the command.
- After Core command state moves to `acknowledged`, the command is no longer
  redispatched even if result/event never arrives.
- Workspace command retries can lose their typed result payload.
- Duplicate workspace commands can repeat side effects if terminal payload is
  not persisted.

Plan:

1. Fix the command state model:
   - transport delivery marker is not execution completion;
   - terminal states are only success/failure/cancelled/expired;
   - `acknowledged` remains retryable until terminal result or lease expiry.
2. Add `dispatch_lease_expires_at` or equivalent deadline for `dispatched` and
   `acknowledged`.
3. Update Core pending-command selection:
   - redispatch expired `dispatched` and `acknowledged`;
   - do not redispatch terminal commands;
   - persist warning/event when redelivering after ACK.
4. Persist typed `CommandResult` payload together with `CommandState` for
   bounded workspace command results.
5. On duplicate `command_id`, return original in-progress or terminal status
   and terminal typed payload without repeating the side effect.
6. Define retention for stored result payloads:
   - keep bounded workspace command results for idempotent replay;
   - defer compaction to a later slice if needed.
7. Add regression tests:
   - ACK received, no result, node reconnects, command becomes dispatchable;
   - ACK received, terminal result received, command is not redispatched;
   - duplicate command does not run workspace write/command twice;
   - retry `ReadWorkspaceFile`/`WriteWorkspaceFile` after state reload returns
     the same typed payload.

Completion criteria:

- Crash after ACK cannot permanently strand a command.
- Command retry behavior is observable through persisted state/events.
- Duplicate workspace command does not lose payload and does not repeat writes.
- Idempotency tests fail on the current implementation and pass after the fix.

## Slice 4: Session Event Projection And Live Updates

Goal: Web session timeline получает все релевантные события exactly-once или
через понятный reload path, regardless of raw event scope.

Findings:

- Events are unique by `(scope_key, seq)`, but session APIs stream by
  `session_thread_id` and raw `seq`.
- Session-scope events can reuse a `seq` lower than runtime-scope events
  already seen by the UI.
- UI computes `after_seq` as max raw event `seq`, so cross-scope events can be
  skipped.
- New empty sessions do not open `EventSource` because the route returns early
  when `events.length === 0`.
- Server emits `uprava.reload`, but the client does not listen for it.

Plan:

1. Introduce a session-level projection cursor:
   - preferred: `session_projection_seq` assigned by Core when an event becomes
     visible in a session timeline;
   - acceptable first step: explicit stream cursor table keyed by
     `session_thread_id`.
2. Change session detail and stream APIs to expose/use the projection cursor,
   not raw per-scope event `seq`.
3. Keep raw `scope_key` and raw `seq` for causality/debugging.
4. Update Web event reducer to order by projection cursor.
5. Open SSE whenever `sessionThreadId` and session data exist; for empty event
   lists use `after_seq=0`.
6. Add `uprava.reload` listener in the SSE client and invalidate/refetch the
   session on reload.
7. Add regression tests:
   - runtime event seq 5 plus session-scope event seq 1 is still delivered;
   - empty session starts `EventSource` with `after_seq=0`;
   - reload event triggers refetch/invalidation.

Completion criteria:

- Session stream cursor is monotonic for the session projection.
- Cross-scope events cannot be skipped by `after_seq`.
- A freshly created empty session receives live events without manual refresh.

## Slice 5: Web Operator Feedback And Visible Error States

Goal: Web Control Panel должен показывать реальные ошибки, preserve user input
on failed sends and avoid stale sensitive UI states.

Findings:

- Failed sends discard the user's draft with no visible error.
- Placement detail hides failed load/start/refresh errors.
- Detail routes mask failed queries as loading or not found.
- Rotated node credentials can remain visible outside the rotation that
  produced them.
- Workspace name slug can suggest current or parent directories.
- Terminal enrollment statuses can be masked by a legacy approved timestamp.

Plan:

1. Change `ChatComposer` contract so `onSend` returns `Promise` or receives
   success/error callbacks.
2. Clear textarea only after successful send.
3. In `SessionRoute`, show `ErrorNotice` for failed `sendTurn`.
4. In detail routes, separate states:
   - loading only while pending;
   - `ErrorNotice` for query failure;
   - not found only after a successful loaded snapshot without entity.
5. In `PlacementRoute`, show errors for failed load, start session and refresh.
6. In `NodeDetailRoute`, show inventory/query errors and avoid false
   `Node not found` before loading completes.
7. Clear `rotatedCredential`:
   - when `nodeId` changes;
   - before a new rotation attempt;
   - after failed rotation.
8. In `PlacementNewRoute`, make slug fallback for `.` and `..` so UI does not
   suggest current or parent directory paths.
9. In `NodeEnrollmentPanel`, check terminal statuses
   `expired`/`revoked`/`rejected` before legacy `approved_at`.
10. Add Vitest coverage for each changed visible state.

Completion criteria:

- Failed send does not clear draft and shows an error.
- Detail routes do not mask API errors as loading/not found.
- Placement and node detail failures are visible to the operator.
- Rotated credential is not visible after failed rotation or node change.
- UI suggestions do not create paths ending in `/.` or `/..`.
- Enrollment terminal statuses are not hidden by legacy approval fields.

## Slice 6: Workspace Inspector Release Baseline And Tool Policy

Goal: зафиксировать Workspace Inspector/command runner as shipped post-`0.1.0`
capabilities in the current `0.1.6` baseline, and align docs, routing,
permissions and UI with that fact.

Findings:

- `docs/en/v01.md` places Project Workspace Inspector, file editor and
  terminal/command runner outside V01, which is correct for the historical
  `0.1.0` product cut.
- `docs/en/releases.md` now treats read-only inspector as `0.1.4` and workspace
  intervention layer as `0.1.5`.
- Server, Node and Web already expose workspace tree/read/write/diff and
  workspace command execution.
- Node validates command shape, but still runs any executable from PATH with
  user-provided args inside an allowed workspace root.

Plan:

1. Keep `V01` as the historical `0.1.0` product cut; do not backport shipped
   post-`0.1.0` feature slices into that scope.
2. Ensure release docs and feature queue agree that:
   - `0.1.4` includes read-only Project Workspace Inspector;
   - `0.1.5` includes workspace text save, bounded command runner, command
     history and diff/check entry points.
3. Update runbook and architecture notes only where they describe current
   implementation behavior, not historical `V01` scope.
4. Define allowed command policy for the current `0.1.6` controlled-dev
   baseline.
5. Record every workspace command/write as command plus event/audit entry.
6. Show command/write risk clearly in UI without relying on hidden agent text.
7. If future deployments need to disable workspace intervention, add a feature
   flag and tests for disabled behavior without redefining `V01`.
8. Add command policy tests:
   - disallowed executable rejected;
   - args/path traversal rejected;
   - disabled feature endpoints return expected error.

Completion criteria:

- Docs and implementation agree that workspace tools are current `0.1.6`
  capabilities, not part of historical `0.1.0`/`V01` scope.
- Command execution is bounded by an explicit policy, not only string cleanup.
- Workspace commands and writes are visible as traceable system actions.

## Slice 7: Runtime Limits, Healthcheck And Logging Hardening

Goal: long-running or large operations should have explicit limits and
recoverable errors instead of unbounded memory use or panics.

Findings:

- Workspace process output limits are applied after unbounded buffering.
- Router uses tracing and CORS layers but no global timeout/backpressure layer.
- Some workspace handlers hold HTTP requests while waiting for Node command
  results up to the workspace intervention timeout.
- Healthcheck stops after the first resolved address.
- `init_tracing` panics instead of reporting subscriber initialization failure.

Plan:

1. Replace `TokioCommand::output()` in workspace process execution with spawned
   process handling and streamed stdout/stderr capped during execution.
2. Ensure timeout kills the process and capped buffers do not keep growing.
3. Add Axum/Tower request timeout and body-size limits appropriate for the
   current `0.1.6` controlled-dev baseline.
4. Decide which workspace operations remain synchronous and which become
   accepted-command plus poll/SSE.
5. Return explicit timeout/cancel states to Web instead of ambiguous failures.
6. Update `uprava-server healthcheck` so it tries all resolved addresses and
   returns success on the first HTTP 200.
7. In `uprava-logging::init_tracing`, replace `.init()` with `.try_init()` and
   return a structured `LoggingError` if subscriber initialization fails.
8. Add tests:
   - high-output helper command does not store bytes beyond cap;
   - command timeout kills child process;
   - request timeout and body-size limits return expected errors;
   - healthcheck helper tries second address after failed first;
   - logging double-init exits normally with error, not panic.

Completion criteria:

- No workspace command can buffer unbounded output in memory.
- Long operations have explicit states and limits.
- HTTP handlers have predictable timeout behavior.
- Healthcheck works with `localhost` resolving to IPv6 and IPv4 in either
  order.
- Logging initialization failure is recoverable through `Result`.

## Slice 8: Schema And Protocol Contract Hygiene

Goal: schema drift and Rust/TypeScript protocol drift fail loudly during local
checks instead of surfacing as runtime UI inconsistencies.

Findings:

- Optional schema migration ignores all `ALTER TABLE` errors.
- Rust protocol and TypeScript protocol types are duplicated manually.
- Web HTTP client casts JSON to `T`; SSE parses JSON without runtime schema.
- Existing tests can use protocol values that TypeScript types do not allow
  because mocks are not validated.

Plan:

1. Replace broad ignored optional schema errors with explicit handling:
   - duplicate column/table is allowed only when expected;
   - other migration errors fail startup/tests.
2. Add schema migration tests for:
   - fresh DB;
   - older partial DB;
   - unexpected broken schema.
3. Choose protocol contract path:
   - generate TypeScript types/schema from Rust; or
   - maintain JSON Schema/OpenAPI as shared source; or
   - interim: checked JSON fixtures emitted by Rust and consumed by Web tests.
4. Add Zod or equivalent validation at high-risk boundaries:
   - session detail;
   - session SSE event;
   - node/placement detail responses.
5. Add client parse error handling for malformed SSE payloads.

Completion criteria:

- Unexpected migration failures are visible.
- Rust and Web protocol changes cannot silently diverge.
- Bad SSE/HTTP payloads become typed UI/API errors instead of reducer crashes.

## Slice 9: Module Boundaries And DDD Cleanup

Goal: reduce hidden coupling by moving Core and Node out of god modules while
keeping behavior stable.

Findings:

- `crates/uprava-server/src/lib.rs` contains routing, config, auth, node
  control, workspace APIs, sessions, events, projections, migrations and tests.
- `crates/uprava-node/src/main.rs` contains config, enrollment, heartbeat,
  WebSocket control, local state, workspace inspector, provider adapter, git
  helpers and tests.
- The main risk is not file size by itself, but the difficulty of reviewing
  transport, persistence and domain state transitions independently.

Plan:

1. Split Core Backend mechanically first, without behavior changes:
   - `config`;
   - `http/router`;
   - `auth`;
   - `node_registry`;
   - `control_channel`;
   - `commands`;
   - `events`;
   - `sessions`;
   - `workspace_api`;
   - `projections`;
   - `db/migrations`;
   - `errors`.
2. Split Node Daemon mechanically:
   - `config`;
   - `state_store`;
   - `enrollment`;
   - `heartbeat`;
   - `control_client`;
   - `workspace`;
   - `providers/codex`;
   - `events/outbox`.
3. Keep public types narrow and avoid broad trait abstractions until tests
   require them.
4. After each mechanical split, run existing tests before behavior changes.
5. Move tests near the module they protect where practical.

Completion criteria:

- Command lifecycle and event stream code can be reviewed without scanning the
  entire server file.
- Node state-store tests do not need provider/workspace test setup.
- Mechanical split produces minimal behavior diff.

## Suggested Execution Order

1. Slice 1 Makefile honesty fixes except final MSRV policy if that needs a
   separate decision.
2. Slice 2 Node allow-list validation, stale control task protection and
   symlink-safe writes.
3. Slice 3 command ACK lease/retry semantics and duplicate payload replay.
4. Slice 4 session projection cursor, empty-session SSE and reload handling.
5. Slice 5 web error states and send draft preservation.
6. Slice 7 bounded output, runtime limits, healthcheck and logging hardening.
7. Slice 1 MSRV policy finalization and local/CI gate update.
8. Slice 6 Workspace Inspector release-baseline docs/config alignment.
9. Slice 8 schema/protocol contract hygiene.
10. Slice 9 module boundary cleanup.

Rationale: first make checks honest, then fix security/correctness/durability,
then clean up product scope and module structure under regression coverage.

## Test Gate

Before handoff for any slice:

```sh
make c
```

If `make c` cannot run because a required scaffolded stack part is absent, the
target must fail with a clear setup message rather than silently skipping that
component. For narrow iteration, use targeted commands first, then run `make c`
before final handoff.

## Global Completion Criteria

This temporary plan is complete when:

- `make c` passes.
- All high findings are fixed or explicitly reclassified with rationale.
- Quality gate findings cannot reproduce.
- Regression tests cover ACK-after-crash retry, command duplicate replay,
  cross-scope session cursor, empty-session SSE subscription, Node state atomic
  write, stale credential save prevention, provider workspace allow-list and
  workspace symlink race.
- User-visible web error/draft-loss findings cannot reproduce.
- Medium command/idempotency/resource findings have regression coverage.
- Current `0.1.6` docs and implemented workspace capabilities no longer
  contradict each other.
- Broad Core/Node module-boundary cleanup, generated protocol contracts and
  async workspace command API are either complete or explicitly moved to the
  feature queue with rationale.
- Remaining low findings are either fixed or moved to the feature queue with an
  explicit reason.
- Durable decisions, especially MSRV policy, Workspace Inspector release scope
  and quality gate behavior, are promoted into canonical docs under `docs/en`
  and `docs/ru` if they outlive this plan.
