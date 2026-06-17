# Codex Runtime Protocol Spike

Date: 2026-06-17

Question: which Codex runtime protocol should the V0.1 Provider Adapter use?

## Local CLI Evidence

- `codex exec --help` exposes non-interactive execution with `--json`,
  `--output-last-message` and `--cd`.
- `codex exec resume --help` exposes non-interactive resume by session id or
  `--last`, also with `--json` and `--output-last-message`.
- `codex app-server --help` exposes an experimental app-server surface with
  `stdio`, `unix` and `ws` transports plus schema generation.
- Generated app-server schemas include richer thread and turn operations, but
  adopting that surface would require a new Node-side live process owner,
  JSON-RPC client, event bridge, approval bridge and lifecycle reconciliation.

## Decision

V0.1 uses the stable `codex exec` / `codex exec resume` path behind the
Provider Adapter boundary. Cortex owns the durable session thread, runtime
projection, ordered events, transcript, provider resume reference and UI
lifecycle state. Codex owns each non-interactive exec/resume invocation.

This satisfies the V0.1 contract as a Core-managed Codex-backed session with
resume continuity. It does not claim provider-native live process ownership,
live output streaming or real Codex interrupt escalation.

## Consequences

- `StartRuntime` creates a Cortex runtime projection and records Codex mode as
  `exec`.
- `SendTurn` uses `codex exec resume` when a provider session id is available.
  Otherwise it builds a bounded prompt from the node-local transcript.
- `ResumeRuntime` restores a persisted provider resume ref when Core has one,
  or falls back to node-local transcript context.
- `StopRuntime` is a Cortex lifecycle transition for the runtime projection.
- `InterruptRuntime` emits an explicit unsupported-provider error for Codex
  exec mode.

## Deferred Work

- Provider-native app-server/live process ownership.
- Incremental streaming output from Codex into Cortex events.
- Real interrupt escalation for running Codex work.
- Structured provider approval and user-input round trips over a live protocol.
