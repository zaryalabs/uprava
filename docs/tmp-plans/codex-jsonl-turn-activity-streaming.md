# Codex JSONL Turn Activity Streaming

Date: 2026-07-06

Status: `active`

Purpose: implement live visibility into Codex work by streaming all available
`codex exec --json` JSONL events from Node Daemon through Core to the Web
Control Panel, rendered as a collapsible `TurnActivity` block inside the
session dialogue.

## Related Docs

- [`docs/en/v01.md`](../en/v01.md) - V01 session, runtime, event and UI
  contracts.
- [`docs/en/architecture.md`](../en/architecture.md) - Core as control plane,
  Node Daemon as data plane and event/log streaming owner.
- [`docs/en/design/003-distributed-runtime-coordination.md`](../en/design/003-distributed-runtime-coordination.md)
  - ordered runtime/session streams and coordination model.
- [`docs/en/design/004-modular-ui-work-surface.md`](../en/design/004-modular-ui-work-surface.md)
  - timeline blocks and future work surfaces.
- [`docs/en/design/008-go-to-source-and-causality-ux.md`](../en/design/008-go-to-source-and-causality-ux.md)
  - source/cause traceability expectations.
- [`docs/tmp-plans/codex-runtime-protocol-spike.md`](codex-runtime-protocol-spike.md)
  - prior decision to keep V0.1 on stable `codex exec` / `codex exec resume`
  instead of adopting provider-native app-server ownership.

## Scope

In scope:

- Node-side live execution for the Codex `exec` adapter.
- Reading `codex exec --json` stdout as JSONL while the process is running.
- Preserving every available field from each Codex JSONL object in Cortex event
  payloads.
- Streaming provider activity events to Core during the turn, not only after
  process completion.
- Rendering provider activity as one collapsible `TurnActivity` block in the
  message history for the matching turn.
- Keeping final assistant output as the existing assistant message path.

Out of scope for this slice:

- Migrating Codex integration to `codex app-server`.
- Provider-native persistent process ownership.
- Full interrupt escalation for a running Codex process.
- Replacing `Message` storage or making provider activity a `Message`.
- Inferring or reconstructing hidden reasoning that Codex does not emit.

## Current Baseline

The protocol already has ordered runtime/session events and a
Core-to-Web session SSE endpoint. The Web Control Panel already subscribes to
`/sessions/{session_thread_id}/stream`.

The missing live-streaming piece is Node Daemon execution. The current Codex
adapter starts `codex exec --json` but uses `command.output()`, so it waits for
the process to finish before parsing stdout and sending a final event batch.
That makes `provider.output.delta` effectively post-hoc instead of live.

## Target Dialogue Shape

The session timeline should render a turn as:

```text
User message
TurnActivity
Assistant message
```

`TurnActivity` is a timeline block, not a `Message`. It is part of the dialogue
because it explains what happened during the agent answer, but it must not
pollute the durable user/assistant message model.

Default UI behavior:

- New/running turn activity starts expanded.
- The block can be collapsed and expanded manually.
- Manual collapse state wins over automatic behavior for the active session
  route.
- After `turn.completed`, the block may auto-collapse only if the user did not
  explicitly expand/collapse it.
- Final assistant text remains a normal assistant message after the activity
  block.

## Event Model

Add a provider activity event kind:

```text
provider.activity
```

Keep the existing:

```text
provider.message.completed
runtime.error
turn.started
turn.completed
```

Do not create a `Message` for `provider.activity`. Core should persist it and
publish it to the session stream, but message insertion remains tied to
`provider.message.completed`, user messages and runtime/approval messages.

Suggested `provider.activity` payload:

```json
{
  "provider": "codex",
  "source": "codex.exec.jsonl",
  "provider_event_type": "item.completed",
  "provider_item_id": "item_3",
  "provider_item_type": "command_execution",
  "phase": "completed",
  "status": "completed",
  "summary": "bash -lc make c",
  "raw_event": {
    "type": "item.completed",
    "item": {
      "id": "item_3",
      "type": "command_execution",
      "command": "bash -lc make c",
      "status": "completed"
    }
  }
}
```

Rules:

- `raw_event` stores the full parsed JSONL object exactly as received from
  Codex, except for bounded size limits if a payload is too large.
- Top-level normalized fields are for filtering, counters, compact summaries
  and future indexing.
- Unknown future Codex fields stay available through `raw_event`.
- If Codex emits reasoning summaries or reasoning items, preserve and render
  them as provider activity. Do not claim access to private chain-of-thought
  that is not emitted.
- If stdout contains a malformed JSONL line, emit bounded provider activity with
  `provider_event_type = "parse_error"`, the raw line snippet and an error
  summary.
- Capture bounded stderr output as provider activity too, because local CLI
  warnings and startup failures can appear there.

## Node Daemon Plan

1. Replace Codex `command.output()` usage with spawned process management:
   `TokioCommand::spawn`, piped stdout and stderr, `kill_on_drop(true)` and the
   existing timeout.
2. Introduce a Node-side event sink for running commands. The sink should:
   - assign runtime sequence numbers;
   - append emitted events to the local outbox;
   - update Node local runtime projection;
   - send small `EventBatch` frames to Core as events are produced;
   - preserve replay behavior for unacknowledged events.
3. Parse Codex stdout as JSONL line by line.
4. For each parsed object, emit `provider.activity` with normalized fields and
   full `raw_event`.
5. Continue writing `--output-last-message` and reading that file after process
   completion for the final assistant message.
6. On successful completion:
   - emit `provider.message.completed` from the last-message file;
   - emit `turn.completed`;
   - emit `runtime.ready` with provider resume reference if discovered.
7. On provider approval request events:
   - preserve the raw JSONL event as `provider.activity`;
   - also emit the normalized `approval.requested` event used by existing UI.
8. On non-zero exit, timeout or spawn failure:
   - emit bounded activity/error context;
   - emit `runtime.error`;
   - mark command result failed.

Open implementation choice: the first iteration can keep command handling
single-flight per control channel. If live send needs mutable access to the
WebSocket while the command runs, split the control channel loop into read and
write tasks with an internal mpsc writer.

## Core Backend Plan

1. Add `ProviderActivity` to `EventKind` with serde name
   `provider.activity`.
2. Persist provider activity events like other events.
3. Publish activity events through the existing session SSE stream.
4. Do not insert rows into `messages` for `provider.activity`.
5. Keep existing projection behavior for:
   - `provider.message.completed` -> assistant message;
   - `approval.requested` -> approval message/projection;
   - `runtime.error` -> runtime message/projection.
6. Optionally expose activity-specific projection helpers later. The first
   slice can let Web build `TurnActivity` from `SessionDetail.events`.

## Web Plan

1. Add a `TurnActivity` timeline block type, likely `core.turn-activity`.
2. Update timeline construction to group activity events by `turn_id`.
3. Insert the grouped block between the user message for that turn and the
   final assistant message when ordering permits.
4. Hide individual provider activity events from the generic event-card path
   once they are included in `TurnActivity`.
5. Render compact rows with:
   - timestamp or relative offset;
   - provider event type;
   - item type;
   - status/phase;
   - summary;
   - optional raw JSON disclosure per row.
6. Render header counters:
   - total events;
   - command/tool calls;
   - file changes;
   - reasoning items/summaries;
   - warnings/errors;
   - duration when `turn.started` and `turn.completed` are both available.
7. Use a bounded scroll region inside the expanded block.
8. Preserve local collapse state by turn id.

The block should be visually quieter than chat messages: compact typography,
monospace where useful, stable dimensions, no nested cards.

## Data Retention And Safety

- Preserve raw provider JSONL objects by default.
- Add conservative bounds before persistence to prevent a single provider event
  from exploding SQLite size.
- If truncation is needed, store:
  - `raw_event_truncated = true`;
  - original byte/char count when known;
  - a bounded preview;
  - enough normalized fields for UI and debugging.
- Do not redact ordinary provider structure silently, but keep existing secret
  handling expectations for logs and command output. If a future redaction layer
  is added, mark redacted fields explicitly.

## Completion Criteria

- Sending a Codex-backed turn shows a `TurnActivity` block before the final
  assistant message.
- The block updates while Codex is still running.
- The block can be collapsed and expanded.
- Raw Codex JSONL fields are visible from the block for each activity row.
- Existing assistant message behavior remains unchanged.
- Reloading the session reconstructs `TurnActivity` from persisted events.
- Sequence gaps still trigger the existing session stream reload behavior.
- Tests cover:
  - Codex JSONL object normalization with unknown fields preserved;
  - Node live event emission order;
  - Core persistence without message insertion for `provider.activity`;
  - Web grouping of multiple provider activity events into one turn block;
  - collapsed/expanded rendering.

## Follow-Up Work

- Move large raw activity streams into log/artifact storage while keeping event
  refs in the timeline.
- Add filters/search inside `TurnActivity`.
- Add provider-specific renderers for common Codex item types.
- Revisit `codex app-server` once Cortex needs real live process ownership,
  richer approvals, steering and interrupt semantics.
