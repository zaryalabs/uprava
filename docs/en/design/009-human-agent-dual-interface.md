# A-009 Human-Agent Dual Interface and Agent as First-Class Citizen

Статус: `working-position`

Этот документ фиксирует рабочую позицию по ключевой механике `A-009 Human-Agent
Dual Interface and Agent as First-Class Citizen`.

Главная позиция: Cortex UI должен быть одновременно удобен человеку и
машинно-читаем для авторизованного агента. Это не означает, что агент смотрит
на скриншот или получает доступ к произвольному DOM. Это означает, что важные
UI objects, artifacts, selections, statuses and actions имеют стабильное
семантическое представление, основанное на Cortex references, permissions,
commands, source/evidence/cause links and runtime state.

Короткая формула:

```text
Every meaningful Cortex UI object should be understandable, referenceable,
navigable and actionable by both a human and an authorized agent.
```

Важное решение: агентский доступ к Core-level UI/context/actions должен
строиться через **Cortex CLI-first interface**, а не через MCP как первичный
контракт. MCP может появиться позже как adapter поверх тех же команд, но
архитектурный source-of-truth для internal agent control должен быть CLI/API
contract, общий с Core command registry and permissions.

## Vision

### Какую проблему решает механика

Обычные agent UI почти всегда строятся вокруг чата. Пользователь видит
визуальную поверхность, но агент часто получает только текстовый prompt и
кусочные вложения. Из-за этого возникают разрывы:

- пользователь говорит "вот этот блок", но агент не знает, какой именно object
  виден и выбран;
- пользователь выделяет текст, но агент не знает source range, host block,
  permissions and nearby context;
- UI показывает disabled action, validation error, warning, diff hunk or chart
  point, но агенту это недоступно без screenshot interpretation;
- агент может написать инструкцию человеку, но не может вызвать безопасную
  registered command над текущим UI context;
- integration behavior прячется в тексте агента вместо того, чтобы быть
  видимым action/event/artifact;
- accessibility для человека и "accessibility" для агента развиваются отдельно,
  хотя им нужны похожие primitives: labels, roles, state, navigation and
  actions.

Cortex должен быть не chat with panels, а shared work surface for human-agent
work. Значит, интерфейс должен быть видимой моделью, с которой работают оба:
человек через visual UI, keyboard, mouse, touch and screen reader; агент через
permission-scoped semantic context and commands.

### Главная модель

У Cortex UI есть две синхронные формы:

```text
Human UI
  visual layout, interaction, focus, menus, popovers, keyboard navigation

Agent-readable UI
  semantic context tree, refs, roles, labels, state, selection, actions,
  permissions, source/evidence/cause links, fallback summaries
```

Agent-readable UI не является:

- raw DOM snapshot;
- screenshot interpretation;
- unrestricted browser automation surface;
- hidden privileged API for agents;
- replacement for accessibility APIs used by humans.

Agent-readable UI является:

- semantic projection of Cortex objects;
- Core-resolvable and permission-scoped;
- built from `Surface`, `Block`, `Artifact`, `Reference`, `Selection`,
  `Command`, `Context`, `NavigableObject`, `Trace/Event` and `Permission`;
- usable by internal agents, CLI clients, future automation and tests;
- explainable to the user through trace and visible action history.

Базовая модель:

```text
Web UI observes current Cortex state
-> surfaces/blocks/renderers expose semantic descriptors
-> Core resolves refs, permissions and command availability
-> UI context snapshot is produced for current user/session/agent scope
-> human invokes context action or agent requests context through CLI/API
-> agent receives structured context, not pixels
-> agent proposes or invokes registered command if allowed
-> Core records action/event/result in trace
-> UI updates visible state and agent-readable state together
```

### Human Agent Interface

Human Agent Interface - это не один виджет. Это набор entry points, где человек
может обратиться к агенту из текущего UI context:

- right-click context menu on object;
- selection popup over selected text/range;
- command palette action;
- keyboard shortcut;
- toolbar/context action in block or inspector;
- chat composer with attached refs;
- mobile long-press/share-style action;
- detail view action such as "Ask about this", "Explain", "Change", "Create
  follow-up task" or "Go to cause".

Все эти entry points должны сходиться в один contract:

```text
current context
+ optional target ref
+ optional selection
+ optional user prompt
-> agent request with structured UI context
```

То есть right-click menu и selection popup являются UX-формами одного
механизма. Они не должны быть единственным способом вызвать агента, иначе
keyboard accessibility, mobile UX and automation будут вторичными.

### Agent as First-Class Citizen

Агент в Cortex должен быть видимым участником системы, а не скрытым процессом
за текстовым ответом.

Для этого UI должен показывать:

- какой agent/session/run сейчас активен;
- какие capabilities ему доступны;
- какой context он получил;
- какие команды он может вызвать;
- какие permissions ограничивают действия;
- когда он ждет человека, tool, node, runtime or permission;
- какие actions были human-initiated, agent-initiated or system-initiated;
- где его output является summary, где tool result, где artifact, где command
  result;
- какие действия требуют approval or review.

Это не значит, что агент становится равен человеку по authority. First-class
citizen означает identity, capabilities, status, context and trace, а не
unrestricted control.

### Почему это отдельное направление

`A-004 Modular UI and work surface` уже задает surfaces, blocks, refs,
commands and context-aware actions.

`A-005 Dynamic UI from agents` задает lifecycle agent-created UI.

`A-006 Visual Rendering and Artifact Semantics` задает source, fallback and
agent-readable visual state.

`A-008 Go to Source and Causality UX` задает навигацию от результата к source,
evidence and cause.

`A-009` нужно выделить отдельно, потому что здесь другой корневой вопрос:

```text
Как человек и агент работают с одной видимой моделью UI, не превращая агента
в скрытый браузерный автокликер и не превращая human UI в недоступный набор
красивых панелей?
```

Если не выделить A-009, появятся два плохих варианта:

- агентский UI context станет набором ad hoc payloads из отдельных panels;
- агент начнет работать через screenshot/DOM automation, обходя Core refs,
  permissions, commands and trace.

Для Cortex оба варианта слабые. Продуктовая сила здесь в том, что UI становится
shared semantic work surface.

## Пользовательские сценарии

### 1. Ask agent about selected text

Пользователь выделяет текст в file viewer, markdown preview, chat message,
terminal output, diff hunk or artifact report и выбирает:

```text
Ask agent about this
```

Cortex передает агенту не только raw selected text:

```text
selection text
selection kind
host block ref
source ref or message range
visible nearby context if allowed
available actions
permissions
related source/evidence/cause refs
```

Агент может ответить текстом, открыть follow-up, предложить edit, перейти к
cause или вызвать allowed command.

### 2. Ask agent from context menu

Пользователь right-click по block, chart point, failed test row, file tree item,
terminal command, diff hunk or warning.

Context menu показывает actions, зависящие от текущего object:

```text
Ask agent about this
Explain
Change this
Go to source
Go to cause
Copy reference
Create follow-up task
```

Каждое действие запускается через command registry. UI не должен собирать
частный prompt вручную; он должен передать structured context.

### 3. Agent explains visible UI state

Пользователь спрашивает:

```text
Почему эта кнопка недоступна?
```

Агент получает current focused object, action availability and disabled reason:

```text
action: apply_patch
enabled: false
reason: permission_denied | stale_workspace | no_selection | validation_error
required_permission: workspace.write
```

Ответ может быть коротким и точным, без догадок по скриншоту.

### 4. Agent operates the workbench through CLI

Агентская runtime среда получает доступ к Cortex CLI. Вместо кликов по
координатам агент делает:

```text
cortex ui context --session <id>
cortex ui actions --ref <ref>
cortex ui invoke ask-agent --ref <ref> --prompt "Explain risk"
cortex ref resolve <ref>
cortex navigate open <ref>
```

Имена команд здесь illustrative. Важно не naming, а решение: stable CLI/API
commands over Core refs and permissions.

### 5. Chat with attached UI context

Пользователь пишет в chat composer:

```text
Почему тут так?
```

Если в UI есть current selection или focused object, composer может показать
attached context chip:

```text
Attached: diff hunk in crates/core/...
```

Пользователь может удалить attachment, добавить другой ref или расширить
context. Это сохраняет явность: агент видит не "весь экран", а выбранный
context package.

### 6. Screen reader and agent-readable labels share discipline

Block renderer должен уметь дать:

```text
human accessible label
keyboard role/focus behavior
agent-readable label
semantic role
state summary
actions
```

Эти слои не обязаны быть одинаковыми строками, но должны описывать один и тот
же UI object. Нельзя делать визуально важный object, который доступен только
мышью или только через private component state.

## Architecture

### Основные сущности

#### UI Context

`UI Context` - snapshot текущей семантической ситуации для user/session/client:

```text
current user
current project/workspace/session
current surface
focused object
active selection
visible objects
open inspectors/detail stack
available commands
permissions
runtime state
redactions
```

UI context должен быть scoped. Агент не должен автоматически получать весь
browser state, все tabs, hidden panels or inaccessible data.

#### Agent-Readable UI Tree

`Agent-readable UI tree` - structured representation visible or relevant UI
objects:

```text
surface
panel
block
artifact
visual object
selection
action
status
warning
validation error
navigation item
```

Минимальный descriptor:

```text
AgentUiObject:
  ref
  kind
  role
  label
  description optional
  state summary
  source_refs
  evidence_refs
  cause_refs
  children optional
  available_actions
  permissions
  fallback_summary
```

Это не обязано быть полным деревом всего экрана. Для многих команд нужен
bounded context package around focused object, selection and visible siblings.

#### Context Package

`Context Package` - payload, который передается агенту при context action:

```text
request_id
origin
user_prompt optional
target_ref optional
selection optional
focused_object optional
visible_context bounded
source/evidence/cause refs
available_actions
permissions
redactions
client/session metadata
```

Он должен быть serializable and traceable. Если часть context redacted or
permission-denied, payload должен сказать об этом явно.

#### Agent Request

`Agent Request` - обращение к агенту, созданное из UI context:

```text
agent_request_id
agent_ref
session_ref
origin command/action
context_package_ref or inline bounded context
prompt
expected mode: answer | propose_change | invoke_action | create_task
trace refs
```

#### Agent Action

`Agent Action` - попытка агента сделать что-то через registered command:

```text
agent_action_id
agent_ref
command_id
target_ref
input
permission decision
result event
```

Agent action не должен обходить Core command registry. Даже если агент
получает context через CLI, privileged work проходит через Core.

#### Cortex CLI

`Cortex CLI` - primary machine interface for agents and automation.

CLI должен быть Rust-based client over Core API, переиспользующий shared API
client and domain types. Возможное имя бинаря (`cortex`, `cortexctl` or
другое) остается naming detail.

CLI отвечает за:

- querying UI context;
- resolving refs;
- listing available actions;
- invoking registered commands;
- opening/navigating refs where a client target is available;
- attaching context to sessions;
- reading trace/event summaries;
- managing auth/session scope for agent runtimes.

MCP adapter может появиться позже, но как secondary adapter:

```text
MCP tool call
-> Cortex CLI/API command
-> Core permission/command registry
-> event/trace/result
```

Нельзя делать MCP source-of-truth для UI semantics, иначе contract будет
зависеть от конкретного agent tool protocol.

### CLI-first decision

Решение: agent access to Cortex Core should be CLI-first.

Причины:

- CLI проще сделать стабильным internal contract для Rust workspace;
- CLI одинаково полезен агентам, людям, scripts, tests and CI;
- CLI может переиспользовать shared API client and type definitions;
- CLI не привязывает Cortex к MCP lifecycle and semantics;
- CLI хорошо ложится на provider adapters, где многие агенты уже умеют
  вызывать shell commands;
- CLI легче версионировать и использовать как debugging surface;
- MCP можно добавить поверх CLI/API без изменения Core model.

Что это не означает:

- MCP не запрещен;
- Cortex не отказывается от MCP integrations;
- external tools/plugins still may expose MCP servers;
- future MCP gateway can exist for compatibility.

Ограничение:

```text
CLI is a control interface, not an authority bypass.
Core still owns permissions, command routing, event log and trace.
```

### Возможный CLI contract

Имена команд preliminary:

```text
cortex ui context
cortex ui tree
cortex ui focused
cortex ui selection
cortex ui actions --ref <ref>
cortex ui invoke <command-id> --ref <ref> --input <json>

cortex ref resolve <ref>
cortex ref open <ref>
cortex ref related <ref>

cortex session context attach <session> <ref>
cortex session ask <session> --ref <ref> --prompt <text>

cortex trace source <ref>
cortex trace cause <ref>
```

Для agent runtime важны machine-readable outputs:

```text
--json
--jsonl for streams
--schema for contract discovery
--bounded / --max-items / --max-bytes
```

CLI должен возвращать explicit errors:

```text
permission_denied
not_found
stale_context
unsupported_client
renderer_unavailable
ambiguous_ref
selection_required
validation_failed
action_requires_approval
```

### UI consequences

#### Context actions are command-backed

Context menu, selection popup, toolbar action, command palette and keyboard
shortcut должны ссылаться на один `command_id`, а не реализовывать разные
локальные flows.

Пример:

```text
command: agent.askAboutContext
target: current ref/selection
input: prompt optional
```

#### Selection is first-class

Selection должна быть semantic, not only browser text selection:

```text
Selection:
  kind: text | range | rows | chart-point | diff-hunk | terminal-output
  host_ref
  source_ref optional
  text/data excerpt bounded
  anchor/focus if useful
  permissions
```

Для text selection важно знать:

- где выделено;
- source range if source-backed;
- какие nearby refs можно безопасно передать;
- является ли текст raw source, rendered markdown, terminal output, generated
  artifact text or external snapshot.

#### Right-click is useful but not canonical

Right-click menu должен быть удобным entry point, но не source-of-truth.

Canonical path:

```text
current context -> command registry -> Core permission check -> event/result
```

Поэтому то же действие должно быть доступно через keyboard and command palette.

#### Agent-readable state must be bounded

Нельзя передавать агенту "весь экран" без границ. Контекст должен быть
bounded by:

- current surface;
- focused object;
- explicit selection;
- visible objects around focus;
- user-selected attachments;
- permission and redaction policy;
- max bytes/items.

#### UI should explain unavailable actions

Если action недоступен, UI and agent-readable state должны иметь reason:

```text
disabled_reason:
  permission_denied
  no_selection
  stale_workspace
  node_offline
  unsupported_object
  approval_required
  validation_error
```

Это улучшает и human accessibility, и agent usefulness.

### Accessibility consequences

Human accessibility:

- real focus management, not visual-only selection;
- keyboard path for every important context action;
- screen reader labels for blocks, actions, warnings and status;
- predictable popover/menu behavior;
- escape paths from menus, editors, terminals and embeds;
- visible focus ring;
- roving tabindex / `aria-activedescendant` where appropriate;
- no mouse-only critical workflows.

Agent accessibility:

- stable refs for meaningful objects;
- semantic roles and labels;
- state summaries;
- source/evidence/cause refs;
- available actions with disabled reasons;
- bounded context snapshots;
- explicit redactions;
- permission-scoped command invocation.

Shared discipline:

```text
If a UI object matters for understanding or action, it needs a human-accessible
path and an agent-readable descriptor.
```

### Permissions and trust

A-009 is security-sensitive because it gives agents more context and possible
action paths.

Rules:

- agent context is scoped to user/session/project permissions;
- agent cannot infer hidden data from redacted descriptors;
- privileged commands still require Core permission checks;
- destructive or external actions can require human approval;
- context packages are traceable;
- agent actions are recorded with agent identity and command input;
- UI should distinguish human action, agent action, system action and external
  integration event;
- stale context must be detected before invoking commands;
- hidden panels, inactive tabs and private client state are not automatically
  shared.

Important boundary:

```text
Agent-readable UI gives understanding and controlled actions.
It does not grant ownership, authority or bypass rights.
```

### Failure modes

| Failure | Expected behavior |
| --- | --- |
| Object has no descriptor | Fallback to parent ref and generic metadata. |
| Renderer cannot expose internal objects | Agent can address outer block only. |
| Selection has no source ref | Include host ref and bounded selected text/data. |
| Ref cannot be resolved | Return explicit `not_found` or `stale_ref`. |
| Permission denied | Redact content and expose denial reason if allowed. |
| Action disabled | Show disabled reason to human and agent. |
| Agent requests hidden context | Deny and record policy event if relevant. |
| CLI cannot reach Core | Return transport error; agent should not pretend context is current. |
| MCP adapter diverges from CLI | CLI/API contract wins. |
| External embed is opaque | Cortex exposes embed boundary, metadata and allowed actions only. |

### Relationship with A-004 Modular UI and Work Surface

`A-004` provides:

```text
surfaces
blocks
artifacts
references
detail views
commands
contributions
context
navigable objects
```

`A-009` says these objects must also expose agent-readable descriptors and
context action entry points when they matter.

### Relationship with A-005 Dynamic UI from Agents

Dynamic UI must be understandable by agents after it is created.

Generated forms, dashboards, charts and artifacts should expose:

```text
component tree or semantic object list
current state
selected object
validation errors
available actions
permissions
source/evidence/cause refs
```

If generated UI is sandboxed executable code, Cortex may only expose the
sandbox boundary unless the artifact package provides an approved semantic
bridge.

### Relationship with A-006 Visual Rendering and Artifact Semantics

A-006 defines how visual objects know source, fallback, actions and agent-
readable visual state.

A-009 uses that state to let the agent answer questions about visible diagrams,
charts, tables, terminal segments, diff markers, test reports and artifact
parts without screenshot parsing.

### Relationship with A-008 Go to Source and Causality UX

A-009 relies on A-008 links:

```text
visible object -> source refs
visible object -> evidence refs
visible object -> cause refs
```

This lets a human ask "почему так?" and lets an agent follow the same chain
through structured refs.

### Relationship with A-007 Plugins, Tool Registry and MCP Strategy

Plugins should be able to contribute:

- commands;
- context menu actions;
- agent-readable descriptors for their blocks/artifacts;
- ref resolvers;
- action handlers;
- redaction policies;
- accessibility metadata.

MCP belongs in A-007 as one integration/tool protocol. For A-009, MCP is not
the primary interface between agents and Cortex Core. The primary interface is:

```text
Core API + Cortex CLI + shared command/ref/context contracts
```

### Quality questions

For every important UI surface/block/action:

- Can a human reach it by keyboard?
- Does it have meaningful accessible labels and focus behavior?
- Does it expose stable Cortex refs?
- Can an authorized agent understand what it is without a screenshot?
- Are available actions command-backed?
- Are disabled actions explained?
- Is current selection semantic and source-aware where possible?
- Is context bounded and permission-scoped?
- Are agent requests/actions traceable?
- Can source/evidence/cause be opened from the same object?
- Does this reduce review and coordination cost compared with chat-only flow?

## Рабочая формула

Human-Agent Dual Interface is the Cortex mechanism for making the same work
surface usable by humans and agents.

Human UI gives visual interaction, keyboard navigation and accessibility.
Agent-readable UI gives semantic context, refs, state, actions, permissions and
causality links. Context actions such as right-click "Ask agent" and selection
popup prompts are entry points into the same command-backed model.

Agent access to Cortex Core should be CLI-first. MCP can be added later as a
compatibility adapter, but the source-of-truth contract should be Core API,
Cortex CLI, command registry, references, permissions and trace.
