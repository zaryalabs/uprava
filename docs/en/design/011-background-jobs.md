# Background Jobs and scheduled agent runs

Status: `working-position`

## Vision

### Problem and product model

The user needs a simple durable form of unattended agent work: run Codex at
night or while working on another project, return later, inspect the result,
and understand what happened. This does not require a visual workflow builder,
an immortal agent process, or a pre-formalized pipeline.

The primary entity is a **Job**, not a Worker. A Job describes work: placement
workspace, prompt/task description, provider launch parameters, and an optional
schedule. The agent/provider executes the Job but is not the Job. Every actual
launch is a separate observable **Job Run**.

The first slice is prompt-first: expected behavior, checks, and results are
mostly expressed in natural language. Add formal restrictions and policy only
when experience demonstrates the need.

### Primary scenarios

- create a paused Job for a concrete placement workspace;
- perform a manual test run and inspect its summary and output;
- enable an interval, daily, or weekly schedule in an IANA timezone;
- launch a Job manually outside its schedule;
- return to history and open a specific run;
- inspect a failed/skipped outcome and decide whether scheduling should resume;
- force-start a chat or Job Run despite a provider quota warning.

### Accepted trust boundary

Provider-native sandboxing and queue item `16a` are not prerequisites. The
current controlled deployment consciously relies on OS user and/or VM isolation
and accepts the risks of unrestricted Codex execution. UI and documentation
must not present this as hostile-workload isolation.

### First-slice scope

Included: one placement per Job; only the existing placement workspace;
paused-by-default creation and manual test-before-enable; manual, interval,
daily, and weekly starts; explicit IANA timezone; run history, summary, and
available provider output/logs; default overlap `skip`; stop-on-error by default
with an opt-out; shared quota admission for chats and Jobs with force override.

Excluded: Git worktrees or an isolated workspace/runtime; arbitrary cron,
event/webhook triggers, and backfill; multi-step pipelines, a workflow canvas,
PR/review loops, complex budgets, and automatic result-quality evaluation.

## Architecture

### Minimal entities

`Job` stores identity, display name, placement, prompt/task description,
provider parameters, enabled state, schedule, overlap policy, and a
continue-after-error flag. A separate immutable revision system is not required
for the first slice; every Job Run retains a snapshot of its effective
configuration so history remains explainable after the Job is edited.

`JobRun` stores the Job reference, configuration snapshot, trigger kind,
timestamps, state, provider/session references, summary, failure, and event/
evidence refs. A separately persisted TriggerOccurrence is not required: a
skipped occurrence can be a compact terminal Job Run without a provider start.

### Lifecycle and scheduling

Minimal Job Run states:

```text
queued -> starting -> running -> succeeded
                     |       -> failed
                     |       -> cancelled
                     |       -> timed_out
                     -> skipped
```

`skipped` represents an expected non-start such as overlap. Provider start or
execution errors produce `failed`. Every terminal reason is typed and visible.

Core owns durable scheduling, atomic claim, and restart recovery. A calendar
schedule is calculated from a local rule plus IANA timezone; the stored due time
is a UTC instant. The default is `continue_after_error = false`: a failed run or
start error pauses later automatic starts. The user can resume the schedule,
perform a manual run, or enable `continue_after_error = true` in advance. An
overlap `skipped` outcome does not pause the schedule.

### Provider quota admission

Quota awareness is a shared provider capability. Before a new interactive
chat/session start or Job Run, Core obtains the last reliable Codex usage
snapshot when the adapter can do so.

- `remaining <= 5%` in either the five-hour or weekly window: typed rejection;
- explicit `force = true`: allow start and record override audit/event evidence;
- no fresh data: quota state is `unknown` and does not block start;
- usage must not be inferred from indirect logs without a reliable provider
  contract.

Implementation first investigates whether the installed Codex CLI exposes a
stable machine-readable source for both limits. The capability and unavailable
reason must be observable.

### Run observation and UX

The Job list shows enabled/schedule state, next start, latest outcome, and an
attention marker. Job detail shows configuration and run history. Run detail
starts with outcome and summary, then exposes timestamps, trigger, effective
configuration, provider/session link, failure reason, and available output/log
stream. Raw output remains a fallback; summary does not replace evidence.

Use the final provider result as the summary. If no structured summary exists,
UI shows terminal assistant output or a missing-summary state without issuing a
hidden second agent call.

### Reuse of the current runtime path

A Job Run creates a separate managed session/runtime execution on the target
placement and uses the normal Core -> Node -> provider path. It does not append
a turn to an arbitrary existing session or introduce a hidden executor.
Correlation connects the Job Run to session events, output, and workspace
evidence.

### Minimal implementation checks

- Core restart does not duplicate a claimed start;
- overlap creates a visible `skipped` outcome;
- a failed run pauses the schedule under the default policy;
- continue-after-error leaves the schedule active;
- manual run works while the schedule is paused;
- history retains the effective configuration snapshot;
- quota threshold blocks chats and Jobs consistently;
- force override leaves audit evidence;
- quota `unknown` does not block start;
- UI exposes summary/output and a typed terminal reason.

