# 2026-07-06 Audit Deferred Follow-Ups

Date: 2026-07-06

Status: `deferred`

Source: carry-over items from `audit-20260706-unified-fix-plan.md` after the
current non-follow-up blockers were implemented and verified.

Purpose: keep only the audit work that was intentionally not implemented in the
current hardening pass, so the unified audit plan can be removed without losing
the remaining roadmap.

## Baseline Already Implemented

Do not carry these items forward from the unified audit plan:

- Quality gate and local MSRV baseline are aligned for the current `0.1.6`
  implementation.
- Node runtime workspace authorization is enforced through canonical
  `UPRAVA_NODE_WORKSPACES` checks for `StartRuntime`, `ResumeRuntime` and
  Codex `SendTurn`.
- Duplicate workspace command replay persists and returns typed result
  payloads after restart.
- Current Node local-state durability, stale credential fencing, symlink-safe
  workspace writes, bounded workspace command output, healthcheck and logging
  hardening are implemented.
- Session projection cursor, empty-session SSE subscription, reload handling
  and visible web error/draft states are implemented.
- Server HTTP request timeout and request-body limits are implemented.
- `make c` passed after the current blocker work.

## Carry-Over Work

### 1. Audit follow-up refactors

Feature queue reference: `docs/en/feature-queue.md` item 17.

Goal: make the command lifecycle, session projection, Node state store,
workspace command execution and web protocol shapes independently testable
without changing shipped behavior.

Scope:

1. Split Core Backend mechanically under current public interfaces:
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
2. Split Node Daemon mechanically under current behavior:
   - `config`;
   - `state_store`;
   - `enrollment`;
   - `heartbeat`;
   - `control_client`;
   - `workspace`;
   - `providers/codex`;
   - `events/outbox`.
3. Keep public types narrow. Do not introduce broad trait abstractions until
   tests or call sites prove the need.
4. Move tests near the module they protect where practical.

Completion criteria:

- Command lifecycle and event-stream code can be reviewed without scanning the
  entire server file.
- Node state-store tests do not need provider/workspace test setup.
- Mechanical splits produce minimal behavior diff.
- `make c` passes after each split.

### 2. Protocol contract and runtime validation

Goal: prevent Rust/Web protocol drift from surfacing as runtime UI
inconsistencies.

Remaining scope:

1. Choose a durable protocol contract path:
   - generate TypeScript types/schema from Rust; or
   - maintain JSON Schema/OpenAPI as the shared source; or
   - use checked JSON fixtures emitted by Rust and consumed by Web tests as an
     interim contract.
2. Add runtime validation at high-risk Web boundaries:
   - session detail;
   - session SSE events;
   - node detail;
   - placement detail;
   - workspace command/history payloads if they remain hand-typed.
3. Add client handling for malformed SSE payloads so parse failures become
   typed UI/API errors instead of reducer crashes.
4. Add any missing migration tests for unexpected broken partial schemas.
   Optional-column duplicate handling is already tightened; do not rework it
   unless a regression test exposes a gap.

Completion criteria:

- Rust and Web protocol changes cannot silently diverge.
- Bad SSE/HTTP payloads become typed client errors.
- Unexpected migration failures fail startup/tests.

### 3. Async workspace command API

Goal: keep the current bounded synchronous workspace operations, but introduce a
clear path for commands that outgrow the synchronous request/response model.

Remaining scope:

1. Decide which workspace operations stay synchronous and which move to
   accepted-command plus poll/SSE.
2. Define API semantics for async workspace commands:
   - command creation;
   - progress/result event stream;
   - timeout/cancel states;
   - durable result lookup;
   - duplicate/idempotent replay behavior.
3. Add Web states for async workspace command progress, timeout, cancellation
   and terminal result.
4. Define compaction/retention for stored Node command result payloads without
   breaking duplicate replay after reconnect.

Completion criteria:

- Long-running workspace operations no longer need to hold HTTP requests for
  their full execution window.
- UI can show progress and terminal states without hidden agent text.
- Result-payload retention is bounded and still idempotent.

### 4. CI and MSRV follow-through

Goal: promote the local `0.1.6` toolchain contract into CI once CI is formalized.

Remaining scope:

1. Add a pinned MSRV check such as
   `cargo +<msrv> check --workspace --all-targets --locked` to CI or the
   canonical local gate.
2. Decide whether a repository `rust-toolchain.toml` is needed, or whether
   documented toolchain requirements are enough.
3. Keep `docs/en/versioning.md`, `docs/ru/versioning.md`, package metadata and
   CI in sync when the Rust version changes.

Completion criteria:

- CI fails when the dependency graph no longer matches the documented Rust
  version.
- Local and CI toolchain expectations are the same.

## Explicitly Out Of Scope

- Production RBAC/permission model.
- SQLite-to-Postgres migration.
- Replacing the Codex provider architecture.
- Broad UI redesign.
- Reopening already-fixed audit blockers unless a new regression reproduces
  them.

## Handoff Gate

Before completing any carry-over slice:

```sh
make c
```

If a slice changes behavior, add focused regression tests before the broad gate.
If a slice is mechanical, keep the diff small and run tests after each
extract/move step.
