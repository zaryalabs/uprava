# A-003 Distributed Runtime Coordination

Статус: `working-position`

Этот документ фиксирует первую рабочую позицию по **Distributed Runtime
Coordination** как отдельной ключевой механике Uprava.

Эта механика находится между `A-001 Distributed Architecture` и `A-002 Run
Mode`:

- `A-001` описывает topology: Core, Node Daemon, clients, registration,
  heartbeat, outbound control channel and node identity.
- `A-002` описывает run semantics: interactive session, runtime strategy,
  turns, process lifecycle, expiry and resurrection.
- `A-003` описывает coordination layer: как Core направляет runtime work на
  конкретную Node/workspace placement, как команды доходят до Node, как events
  возвращаются в Core, как система переживает stale/offline/retry, и как UI
  показывает resource warnings.

V01 не должен превращать это в workflow engine, scheduler or lock manager.
Baseline простой: Core показывает inventory tree `Nodes -> Projects/Workspaces`,
dispatch-ит commands через Node control channel, принимает ordered events,
строит status projections и показывает warning badges там, где работа может
конфликтовать с уже известными ресурсами.

## Vision

### Какую проблему решает механика

Distributed architecture дает Uprava возможность видеть ноды и открывать
control channel. Run Mode дает модель рабочей сессии и runtime lifecycle.
Между ними остается важный вопрос: **как конкретная agent work единица
координируется в распределенной системе**.

Без отдельной coordination-механики быстро смешиваются разные темы:

- network presence и node auth начинают описывать runtime lifecycle;
- run mode начинает описывать routing, retry, stale node behavior and resource
  conflicts;
- UI не знает, где показывать предупреждения: в node tree, перед start, в
  session header или внутри trace;
- будущие sandbox/task runs получают отдельный dispatch path вместо общего
  coordination layer.

Цель A-003 - зафиксировать общий слой, который будет использоваться и первой
interactive session, и будущими task/sandbox runs.

### Концептуально как реализуем

V01 coordination строится вокруг пяти идей.

1. **Node-centric inventory tree**. Core показывает пользователю дерево
   `Nodes -> Projects/Workspaces`. Элемент под Node - это конкретная
   node-local project/workspace placement, а не обязательно новый global
   project.
2. **RuntimeSession as coordination unit**. Команды и events координируются
   вокруг `RuntimeSession`, которая привязана к `SessionThread` и
   `ProjectPlacement`.
3. **Command proxy, not workflow scheduler**. Core dispatch-ит команды через
   active outbound control channel. Если channel закрыт, Core просит Node
   открыть его через heartbeat response. V01 не делает полноценный
   desired-state reconciler.
4. **Resource awareness through badges**. Core хранит минимальные resource
   signals и показывает warning badges для потенциальных конфликтов. Hard block
   только там, где действие реально невозможно.
5. **Idempotent commands and ordered events**. Commands имеют `command_id`,
   events имеют `event_id` and per-runtime `seq`, чтобы retry/reconnect не
   создавали duplicate turns или неконсистентный UI.

### Пользовательские сценарии

#### 1. Выбор места запуска

Пользователь открывает workbench и видит дерево:

```text
MacBook online
  uprava / main / dirty / 1 active session
  site / develop / clean

VPS online
  uprava / main / clean / warning: same branch active on MacBook

Devbox stale
  uprava / feature-x / last seen 2h ago
```

Core не выбирает Node автоматически. Он помогает пользователю выбрать
конкретный placement: показывает online/stale/offline, capabilities,
workspace/git state and badges.

#### 2. Start runtime через coordination layer

Пользователь выбирает placement и нажимает Start.

Core:

1. проверяет, что Node reachable or recoverable;
2. проверяет, что workspace exists/writable;
3. смотрит resource signals;
4. показывает warning badges, если есть риск;
5. при подтверждении dispatch-ит `StartRuntime` на Node;
6. строит session projection из incoming events.

#### 3. Warning, но не hard block

Если Core видит тот же repo/branch на другой Node с active runtime, UI
показывает badge: `same branch active elsewhere`. Пользователь может все равно
продолжить. Override пишется в event log как
`coordination.warning_acknowledged`.

#### 4. Node stale/offline

Если Node `stale`, Core показывает last known state and badges, но не отправляет
новый turn без восстановления channel. Если Node `offline`, runtime commands
blocked, но historical session остается readable.

#### 5. Future sandbox/task reuse

Когда появятся bounded tasks or sandboxed runtime, они должны использовать тот
же coordination layer: command dispatch, event ordering, resource signals,
warnings, overrides and node status. Меняется work contract/runtime strategy,
а не базовая distributed coordination.

### Agent-facing сценарии

Worker agent не должен думать о distributed coordination. Для него есть
workspace, tools, process environment and provider runtime.

Coordination layer работает вокруг agent:

- выбирает placement до запуска runtime;
- проверяет resource signals;
- route-ит commands/events;
- сохраняет trace of warnings and overrides;
- показывает UI status, если Node stale/offline/resuming.

Если coordination issue влияет на runtime, agent получает обычную ошибку
runtime/tool execution или явный resume context. Подробная node topology не
становится частью базового prompt/context.

### First release vs later

#### V01

Нужно:

- `Nodes -> Projects/Workspaces` inventory tree;
- explicit user choice of placement;
- `RuntimeSession` as coordination unit;
- command proxy через active outbound control channel;
- heartbeat-triggered control channel open request;
- idempotent commands with `command_id`;
- events ordered by `runtime_session_id + seq`;
- Core projections by `session_thread_id`;
- minimum resource signals;
- warning badges and explicit override;
- event log entry for warning acknowledgement;
- stale/offline behavior;
- no locks by default.

V01 не требует:

- auto-scheduling;
- global desired-state reconciler;
- distributed locks;
- queue system;
- multi-node orchestration;
- hard blocking on git branch collisions;
- generic resource optimizer.

#### Later

Позже можно добавить:

- Core recommendations for best Node placement;
- configurable resource policies;
- stronger locks for team/cloud modes;
- task/sandbox-specific placement policies;
- node capacity planning;
- queueing and backpressure;
- branch/worktree creation actions;
- richer conflict explanations and remediation.

## Architecture

### Current agreed baseline

1. **Core coordinates full runtime lifecycle**: `start`, `send turn`, `stream
   events`, `block`, `interrupt`, `stop`, `expire`, `resume`, `reconcile`.
2. **Coordination unit is `RuntimeSession`**, linked to `SessionThread` and
   `ProjectPlacement`.
3. **V01 is command proxy plus persisted event log**, not a full
   desired-state reconciler.
4. **Core shows `Nodes -> Projects/Workspaces` tree**. User chooses concrete
   project placement on a Node. Core shows status/capabilities/warning badges.
5. **Minimum resource signals**: `node_status`, `workspace_exists`,
   `workspace_writable`, `git_repo_url`, `git_branch`, `git_commit`,
   `git_dirty`, `active_runtime_on_workspace`,
   `active_runtime_on_same_repo_branch`.
6. **Hard block only for impossible actions**: Node offline, workspace missing
   or unwritable, permission denied. Same repo/branch, dirty state and stale
   runtime are warning badges with override.
7. **Command dispatch uses outbound control channel**. If channel is closed,
   Core asks Node to open it through heartbeat response.
8. **Idempotency is mandatory**. Commands have `command_id`; stable target ids
   are used for `StartRuntime`, `ResumeRuntime` and `SendTurn`. Node
   deduplicates by `command_id`; Core deduplicates events by `event_id + seq`.
9. **Event ordering is per `runtime_session_id`** through monotonic `seq`. Core
   builds UI projection per `session_thread_id`.
10. **Node stale/offline behavior is simple**. `stale` shows last known state
    and allows wait/reconnect; new turn waits for restored channel. `offline`
    blocks runtime commands; historical session remains readable.
11. **Node owns runtime process handle**. Core stores projection, ids, status,
    events and resume reference. Browser never connects directly to process.
12. **Warnings are badges** in inventory tree, before start/resume and in
    session header.
13. **Warning override is allowed** and stored as
    `coordination.warning_acknowledged` with actor, warning kind and affected
    resources.
14. **Future tasks/sandbox runs reuse this layer**. They add another work
    contract/runtime strategy, not a separate dispatch system.
15. **Git-specific baseline is a warning signal set**, not lock semantics:
    `repo_url`, normalized `repo_id`, `branch`, `commit`, `dirty`, `ahead`,
    `behind`, `untracked_count`, `active_runtime_count`.

### Core inventory model

Core distinguishes logical project identity from node-local placement. For the
0.2.0 protocol this is a fixed identity contract, not an open implementation
choice.

```text
Project
  project_id
  display_name
  repo_id optional

Node
  node_id
  display_name
  status
  capabilities

ProjectPlacement
  placement_id
  project_id optional while unbound
  node_id
  canonical_workspace_path
  git_snapshot optional
  runtime_summary
  last_seen_at
```

`Project` is a Core-owned logical aggregate whose identity is independent of a
Node or path. `ProjectPlacement` is the physical Node/path binding, and the
database enforces one Placement per `(node_id, canonical_workspace_path)`.
One Project may own Placements on multiple Nodes. Node reports canonical local
facts but does not mint Project or Placement identifiers.

Heartbeat discovery and explicit binding converge on the same physical
Placement. Discovery creates or refreshes one unbound Placement; binding
attaches it to a selected or newly created Project. Core never infers
cross-node Project grouping from a path alone.

`Workspace` is the user-facing workbench over a Placement, not a second
persisted entity. The Core resource remains `/placements/:id`; the canonical
Web surface is `/workspaces/:placement_id`. UI may still render the inventory
as `Nodes -> Projects/Workspaces`, but `Workspace` does not introduce another
domain identifier.

### Resource signals

Resource signals are small, current-enough facts used to render badges and make
start/resume safer.

Minimum V01 signals:

```text
node_status
workspace_exists
workspace_writable
git_repo_url
git_repo_id
git_branch
git_commit
git_dirty
git_ahead
git_behind
git_untracked_count
active_runtime_on_workspace
active_runtime_on_same_repo_branch
```

Signals can be stale. UI should show freshness when it matters, especially for
offline/stale Node.

### Warning badges

Badges are lightweight coordination feedback, not a policy engine.

Initial badges:

| Badge | Trigger | Default action |
| --- | --- | --- |
| `offline` | Node offline | Hard block runtime commands. |
| `stale` | Node heartbeat stale | Warn, wait/reconnect before new turn. |
| `missing workspace` | Workspace path unavailable | Hard block start/resume. |
| `read-only workspace` | Workspace not writable | Hard block write-capable runtime. |
| `dirty` | Git working tree dirty | Warn and allow override. |
| `same branch active` | Same normalized repo/branch has active runtime elsewhere | Warn and allow override. |
| `same workspace active` | Same workspace already has active runtime | Warn and allow override in V01. |
| `behind remote` | Branch behind upstream | Warn only. |
| `missing capability` | Node lacks required capability/tool | Hard block if required, warning if optional. |

Override event:

```text
coordination.warning_acknowledged {
  actor_id
  warning_kind
  affected_resources
  command_id
  happened_at
}
```

### Command dispatch

V01 command flow:

```text
UI -> Core command request
Core validates placement and resource signals
Core records command intent
Core sends command over Node control channel
Node deduplicates by command_id
Node executes or rejects with reason
Node streams ordered events to Core
Core updates projection and UI stream
```

If control channel is closed:

```text
Core marks channel_needed for Node
Node sees request in heartbeat response
Node opens outbound control channel
Core dispatches queued command if still valid
```

This is command proxy with retry, not a queue/scheduler product.

### Event ordering

Node emits events with:

```text
event_id
command_id optional
runtime_session_id
session_thread_id
seq
kind
happened_at
payload
```

Rules:

- `seq` is monotonic per `runtime_session_id`;
- Core deduplicates by `event_id`;
- Core can detect gaps by `seq`;
- UI projection is built per `session_thread_id`;
- V01 does not need global event ordering across all nodes.

### Stale and offline

`stale` means Core has not received fresh heartbeat/control-channel state. It
does not prove the Node is dead.

V01 behavior:

- keep historical session readable;
- show last known node/runtime/resource state;
- show stale badge in tree and session header;
- do not send new turns until channel is restored;
- allow user to wait, retry reconnect, or inspect last known state.

`offline` means Core considers Node unavailable for runtime commands. Historical
sessions remain readable. Resume/start on that Node is blocked until Node
returns.

### Git repo awareness

Git awareness is one resource signal family inside Distributed Runtime
Coordination. It is not the whole mechanism.

V01 should detect:

- same normalized repo id;
- same branch;
- active runtime count on same branch;
- dirty workspace;
- ahead/behind;
- untracked count.

Same repo/branch on multiple nodes should produce a badge, not a hard lock.
The warning exists because two live runtimes can edit divergent local copies of
the same logical branch. Uprava should make that visible before start/resume.

No V01 locks:

- no global branch lock;
- no forced checkout;
- no automatic push/pull;
- no automatic worktree creation;
- no merge/conflict resolution.

Later versions can add safer actions: create branch, create worktree, stop
other runtime, sync remote, or apply team policy.

### V01 implementation checklist

- Add Core model for `ProjectPlacement` or equivalent node-local workspace
  inventory item.
- Add Node workspace scanner or explicit workspace registration.
- Add minimal git snapshot collection in Node.
- Add inventory tree endpoint for `Nodes -> Projects/Workspaces`.
- Add resource badges to inventory projection.
- Add command envelope with `command_id`.
- Add Node command deduplication by `command_id`.
- Add event envelope with `event_id` and per-runtime `seq`.
- Add Core event deduplication and per-session projection.
- Add warning acknowledgement event.
- Add start/resume preflight that returns hard blocks and warning badges.
- Add stale/offline behavior for runtime commands.

### Remaining architecture questions

- Should V01 discover workspaces automatically, or should users register
  workspace roots/projects explicitly?
- How should Core normalize git remote URLs into stable `repo_id`?
- Which resource signals are refreshed periodically, and which only on
  preflight?
- Should same workspace active runtime be warning or hard block for the first
  prototype?
- How much event gap recovery do we need in V01?
