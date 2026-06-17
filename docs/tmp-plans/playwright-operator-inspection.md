# Playwright CLI Operator Inspection

Date: 2026-06-17.

Purpose: record a manual/agent `playwright-cli` inspection against the
source-built local profile for the `0.1` release audit.

## Profile

- Core: `127.0.0.1:19280`, `local_trusted`,
  `CORTEX_AUTO_APPROVE_ENROLLMENTS=true`.
- Web: `127.0.0.1:15176`, `VITE_CORTEX_API_BASE` pointed at the temporary
  Core API.
- Node: `Operator Check Node`, workspace
  `/private/tmp/cortex-operator-check.SmoiVT/workspace`.
- Provider: fake provider.

The browser was driven with:

```sh
playwright-cli -s=cortex-operator open about:blank
playwright-cli -s=cortex-operator goto http://127.0.0.1:15176
```

Raw snapshots were generated under `.playwright-cli/` during the run and are
distilled here so generated browser artifacts do not need to be kept in git.

## Observed

- App shell showed `API connected`.
- Trusted/local banner was visible:
  `local_trusted profile · trusted local or controlled development use only`.
- Inventory tree showed `Operator Check Node` as `reachable`, with idle/active
  state, `sleep awake`, provider capabilities and a validated workspace.
- Nodes route showed the trusted pairing copy and disabled approve button for an
  already claimed auto-approved enrollment.
- Placement route showed the validated workspace path, provider selector,
  placement reference actions, refresh and `Start`.
- Starting the fake session navigated to
  `/sessions/41d88367-bb31-4503-822c-8f413f1b9df0`.
- Reloading the session reconstructed persisted runtime state and events from
  Core: `runtime.starting` seq 1 and `runtime.ready` seq 2.
- Sending `operator browser verification` through the composer rendered a user
  message, an assistant message with
  `Fake provider accepted: operator browser verification`, ordered
  runtime/turn/provider events and an agent projection count of
  `2 messages, 8 events, 0 pending approvals`.
- Detach changed the session to `detached`, enabled `Attach`, disabled
  `Detach`, and disabled the composer/send controls.
- Attach restored `active` state and re-enabled the composer.
- Stop changed the session/runtime badges to `stopped`, disabled
  attach/detach/stop and enabled `Resume`.
- Resume returned the same session to `active`/`ready` and appended
  `runtime.resuming` and `runtime.ready`.
- Sending `operator browser verification after resume` rendered the post-resume
  user/assistant messages and increased the projection to
  `4 messages, 17 events, 0 pending approvals`.
- Reloading after the post-resume turn reconstructed the same messages,
  runtime events, artifact tree and agent projection from Core persistence.
- Opening the session reference in the Inspector added an `inspect=` URL state
  and showed session id, state, runtime state, provider and message/event
  counts.
- Sending `/error operator visible error` produced visible degraded/error UI:
  the inventory row changed to `Session for workspace degraded error`, the main
  runtime badge showed `error`, the runtime error block rendered
  `operator visible error`, and the Inspector runtime state showed `error`.
- Resuming from the error returned the session to `active`/`ready`; the
  Inspector runtime state showed `ready` and the projection showed
  `6 messages, 22 events, 0 pending approvals`.

Console output only contained missing `/favicon.ico` 404s.
