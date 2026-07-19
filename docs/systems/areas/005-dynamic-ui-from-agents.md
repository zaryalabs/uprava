# A-005 Dynamic UI from Agents

Статус: `working-position`

Этот документ фиксирует рабочую позицию по ключевой механике `A-005 Dynamic
UI from Agents`.

Ключевая позиция: `A-005` не является отдельной UI-системой рядом с `A-004
Modular UI and work surface`. Это отдельное функциональное направление внутри
модели A-004.

`A-004` отвечает за то, где живут surfaces, blocks, artifacts, references,
detail views, commands, contributions and renderers. `A-005` отвечает за
особый класс блоков и artifacts: UI, который появляется из работы агента,
tool-а или plugin-а, но монтируется в те же Uprava surfaces и живет по тем же
правилам permissions, trace, fallback and review.

Implementation direction для этого направления — bundled first-party plugin,
а не новый hardcoded subsystem в основном React tree. Base Uprava предоставляет
generic proposal/artifact contracts, validation, persistence, permissions,
command/event routing, sandbox boundary and fallback. Dynamic UI plugin
предоставляет component catalogs, renderers and related contributions через
общий Plugin Registry/Extension Host contract. По мере развития A-005 этот
contract должен становиться пригодным для local/team/community plugins, как в
extension ecosystems Obsidian и VS Code.

Документ намеренно не описывает, что попадет в первую или вторую версию
продукта. Scope конкретных итераций должен определяться отдельно. Здесь
проектируется целое направление: какие формы dynamic UI нужны Uprava, как они
связаны с agent work, где проходит граница безопасности, и какие базовые
архитектурные контракты нельзя сломать будущими реализациями.

## Vision

### Какую проблему решает механика

Agent output не должен быть ограничен текстом. Во многих сценариях текст
является худшей формой ответа:

- агент собрал данные, но пользователь должен сравнивать их в таблице;
- агент сделал проверку, но результат лучше смотреть как report with filters;
- агент предлагает выбор, но пользователю нужна форма с constraints and
  validation;
- агент анализирует систему, но вывод лучше понимать как graph, dashboard,
  timeline or map;
- агент строит финансовую, инженерную или продуктовую модель, но ей нужен
  interactive calculator;
- агент работает с внешней системой, но integration behavior должно быть
  видно как UI block, event and command, а не как скрытый API call внутри
  текста.

Обычный чат плохо поддерживает такие сценарии. Пользователь вынужден читать
длинное объяснение, копировать данные в сторонние инструменты, вручную
проверять аргументы tool calls и реконструировать состояние задачи.

Dynamic UI должен снизить стоимость понимания, review and correction. Uprava
должен позволить агенту не только сказать "вот результат", но и породить
структурированный, интерактивный объект, с которым пользователь может работать:
смотреть, фильтровать, подтверждать, редактировать, запускать команды и
переходить к причинам.

### Главная модель

Dynamic UI в Uprava - это не "агент генерирует произвольный React в чат".

Базовая модель:

```text
agent/tool/plugin proposes UI intent or artifact
-> Core validates type, schema, refs, permissions and trace metadata
-> Core stores block/artifact/event metadata
-> Web mounts the UI through an approved renderer/runtime
-> user interaction becomes command/action/event
-> Core routes action through registry and permissions
-> result updates artifact, block, trace or workflow state
```

Агент может:

- выбрать подходящий зарегистрированный visual block;
- заполнить typed props/data для renderer-а;
- предложить declarative UI schema;
- создать generated UI artifact;
- обновлять data model or artifact snapshot;
- запросить user action через форму, control, approval или command;
- связать UI с trace refs, files, commands, tool calls and artifacts.

Агент не должен автоматически получать право:

- выполнять произвольный JavaScript в основном React tree;
- обходить Core permissions;
- вызывать privileged commands напрямую из generated UI;
- скрывать external API calls за визуальным элементом;
- хранить важное состояние только внутри ephemeral frontend component;
- создавать UI, который невозможно прочитать как artifact/fallback.

Короткая формула:

```text
Agent can propose and parameterize UI.
Core decides whether it is allowed.
Web renders through known contracts.
User actions return as traceable commands/events.
```

### Почему это отдельное направление

Dynamic UI является частным случаем blocks/artifacts из A-004, но его нужно
выделить отдельно, потому что здесь появляются отдельные продуктовые and
архитектурные вопросы:

- кто может породить новый UI на лету;
- чем tool-rendered UI отличается от generated app artifact;
- где проходит граница между declarative schema and executable code;
- как сохранять dynamic UI в истории session/thread;
- как делать fallback, если renderer недоступен;
- как user actions из generated UI превращаются в commands/events;
- как агент понимает UI state, который сам породил;
- как не потерять traceability, permissions and reviewability.

Если не выделить A-005, dynamic UI легко растворится в frontend abstractions.
Тогда появится риск, что каждый renderer, integration или artifact начнет жить
по своим правилам. Для Uprava это плохо: dynamic UI должен быть частью control
plane, event log, Tool Registry, Plugin Registry, artifact model and causality
model.

### Два базовых типа dynamic UI

Внутри A-005 нужно различать два крупных типа. Они оба являются блоками или
artifacts внутри A-004, но имеют разную природу.

#### Tool-rendered block

Tool-rendered block появляется как visual representation зарегистрированного
tool call, command, integration action или structured result.

Примеры:

- `run_tests` возвращает test report block;
- `search_github_issues` возвращает issue list block;
- `deploy_preview` возвращает deploy status block;
- `query_metrics` возвращает chart/table block;
- `request_approval` возвращает approval control;
- `create_form_request` возвращает маленькую typed form для уточнения input.

Здесь UI не является самостоятельным приложением. Он является представлением
известного действия или результата:

```text
registered tool/command
-> typed input/output schema
-> renderer_id
-> block props/data
-> actions mapped back to commands
```

Это устойчивый паттерн для большого числа developer-workbench and
integration scenarios, потому что он:

- прозрачен для trace;
- безопасен;
- хорошо ложится на Tool Registry;
- хорошо типизируется;
- легко имеет fallback;
- позволяет показывать progress/status live;
- делает tool-heavy agent work понятнее без произвольного UI runtime.

#### Generated app/artifact block

Generated app/artifact block - более самостоятельный interactive artifact,
созданный агентом для конкретной задачи.

Примеры:

- dashboard с filters and drilldowns;
- wizard/form для сбора требований;
- dependency graph с выбором узлов;
- calculator сложного процента;
- simulator для capacity planning;
- interactive checklist/review tool;
- small decision model with editable assumptions;
- generated report с charts, controls and persisted state.

Такой объект уже не просто "карточка результата tool call". Это durable
artifact, у которого есть:

- artifact identity;
- schema or package;
- data model;
- renderer/runtime;
- version history;
- permissions;
- action bridge;
- fallback representation;
- trace refs to source agent work;
- persistence and restore semantics.

Внутри этого типа нужно различать две формы.

#### Schema-driven generated UI

Schema-driven generated UI подходит для forms, dashboards, charts, tables,
reports, selectors, wizards and simple interactive controls.

Агент генерирует не code, а declarative description:

```text
surface
component tree
data model
bindings
actions
constraints
validation rules
trace refs
```

Web рендерит это через Uprava component catalog. Визуальный стиль, layout
constraints, accessibility, commands and permissions остаются под контролем
Uprava.

Свойства:

- no arbitrary code execution;
- portable across web/mobile/future clients;
- easier fallback;
- easier validation;
- better consistency with design system;
- suitable for agent-readable UI state.

Для Uprava это должен быть основной путь для generated forms and dashboards.

#### Sandboxed app artifact

Sandboxed app artifact нужен там, где declarative schema недостаточно:
сложные calculators, simulations, visual editors, interactive playgrounds,
custom generated apps.

Здесь допускается executable UI, но только в изолированном runtime:

```text
artifact package
-> sandboxed iframe or equivalent isolated runtime
-> strict CSP/capability policy
-> message/action bridge to Core
-> no direct privileged access
-> persisted artifact state outside sandbox
-> safe fallback snapshot
```

Такой artifact ближе к Anthropic Artifacts-like модели: пользователь получает
не просто блок в чате, а маленькое приложение или interactive object в
отдельной artifact surface.

Но даже в этом режиме Uprava не должен превращаться в "браузер внутри
браузера", где generated code живет без governance. Sandboxed app artifact
должен быть artifact-centered and sandbox-contained.

### Пользовательские сценарии

#### 1. Агент показывает результат tool call визуально

Пользователь просит: "Посмотри, почему тесты падают".

Агент запускает проверку через tool. Вместо того чтобы вернуть только текст,
Uprava показывает block:

```text
Test report
  failed: 3
  passed: 212
  slow: 8
  actions: open failed test, rerun failed, copy command, go to cause
```

Block связан с command output, files, trace events and diff. Пользователь
может перейти от failed test к source location или к tool invocation.

#### 2. Агент просит уточнить параметры через форму

Пользователь просит: "Подготовь миграцию для новой billing model".

Агент понимает, что нужно уточнить параметры. Вместо длинного списка вопросов
он создает form block:

```text
Billing migration inputs
  current plans
  target plans
  rollout mode
  migration date
  risk tolerance
  required checks
```

Пользователь заполняет форму, Core валидирует fields, сохраняет submission as
event, и агент получает structured input.

#### 3. Агент создает dashboard artifact

Пользователь просит: "Собери dashboard по состоянию проекта".

Агент агрегирует events, checks, diff, active sessions, failing tasks and
resource warnings. Uprava показывает generated dashboard artifact:

```text
Project status dashboard
  active work by node
  recent failures
  open approvals
  risky diffs
  check trend
  artifacts needing review
```

Dashboard не является скрытым frontend state. Это artifact with data snapshot,
sources, trace refs and actions.

#### 4. Агент создает interactive calculator

Пользователь просит: "Смоделируй стоимость запуска 20 агентов на разных
нодах".

Агент создает calculator artifact с assumptions, sliders and charts.
Пользователь меняет количество агентов, runtime duration, hardware profile and
provider pricing. Artifact пересчитывает результат локально в sandbox, но
сохранение новой версии, export and commands идут через Core.

#### 5. Агент обновляет существующий UI artifact

Пользователь возвращается к dashboard через неделю. Агент не генерирует новый
объект с нуля, а обновляет data model and references existing artifact:

```text
artifact_id: project-dashboard-123
update:
  data_model.checks = latest checks
  data_model.sessions = current sessions
  trace_refs += new events
```

UI сохраняет continuity: пользователь видит тот же artifact, его историю,
версии и изменившиеся данные.

### Agent-facing сценарии

Агент должен иметь машинно-понятный способ узнать, какой dynamic UI он может
создать.

Core может предоставить agent context:

```text
available_dynamic_ui:
  - renderer_id: test-report
    kind: tool-rendered-block
    input_schema: ...
    allowed_surfaces: [session.timeline, artifact.viewer]
  - renderer_id: basic-dashboard
    kind: schema-driven-ui
    component_catalog: uprava.basic
    max_components: ...
  - renderer_id: form
    kind: schema-driven-ui
    supported_fields: ...
```

Агент не должен угадывать frontend internals. Он должен работать с
capabilities:

- "можно показать chart";
- "можно создать form";
- "можно открыть generated dashboard artifact";
- "можно запросить approval";
- "можно обновить существующий artifact";
- "нельзя использовать sandboxed app runtime в текущем trust scope".

Для agent-facing модели важно, чтобы UI был не только видимым человеку, но и
readable by agent. Dynamic UI должен иметь machine-readable representation:
components, state, selection, actions, refs, permissions and validation errors.

## Architecture

### Relationship with A-004

A-005 использует сущности A-004:

```text
Surface
Block
Artifact
Reference
Detail View
Aspect
Command
Contribution
Context
Service
Plugin
Navigable Object
```

И добавляет dynamic-specific сущности:

```text
Dynamic UI Proposal
Dynamic Block
Generated UI Artifact
Renderer Contract
Component Catalog
Data Model
Binding
Action Bridge
Sandbox Runtime
Fallback Representation
UI Capability
UI Trust Level
```

Граница:

```text
A-004 answers:
  where blocks/artifacts/renderers/actions live in the workbench

A-005 answers:
  who can create dynamic blocks/artifacts
  which dynamic forms exist
  how they are validated, rendered, persisted and acted upon
```

Dynamic UI block должен быть обычным участником A-004:

- он находится в known surface;
- имеет address/reference;
- поддерживает navigation model where possible;
- может иметь detail view;
- имеет command/actions;
- имеет fallback;
- связан с trace refs;
- подчиняется Core permissions.

### Core principles

#### 1. Dynamic UI is artifact/event-backed

Важный dynamic UI не должен существовать только как ephemeral frontend state.
Если UI влияет на работу, review, decision, command или future context, он
должен быть backed by event/artifact metadata.

```text
visible dynamic UI
-> durable block/artifact descriptor
-> event log entry
-> trace refs
-> fallback representation
```

#### 2. Generated UI is untrusted until validated

Agent output является untrusted input. Это относится и к text, и к JSON, и к
generated UI schema, и к generated app package.

Core должен валидировать:

- type;
- schema version;
- renderer availability;
- surface eligibility;
- data size;
- references;
- permissions;
- actions;
- external origins;
- sandbox capabilities;
- persistence policy.

#### 3. Actions are commands, not callbacks with hidden power

User action внутри dynamic UI не должен напрямую выполнять privileged effect.
Он должен превращаться в Core-visible command/action event.

```text
button click
-> DynamicUiActionRequested
-> permission check
-> command dispatch or agent input
-> result event
-> block/artifact update
```

Даже если action выглядит как локальная кнопка, продуктовая семантика должна
быть traceable.

#### 4. Renderer and execution trust are separate

То, что UI красиво отрендерился, не означает, что он получил право выполнять
действия. Нужно разделять:

```text
render permission
interaction permission
command permission
external access permission
sandbox capability
```

Например, dashboard может быть доступен read-only, форма может быть editable
but not submittable, sandboxed app может делать local calculations but cannot
call external network.

#### 5. Fallback is mandatory

Каждый dynamic block/artifact должен иметь safe fallback.

Fallback нужен, если:

- renderer отсутствует;
- plugin disabled;
- schema version unsupported;
- sandbox blocked;
- permission denied;
- mobile client cannot render full UI;
- artifact package failed validation;
- external embed unavailable.

Fallback может быть:

```text
metadata card
raw sanitized structured data
static snapshot
markdown/table summary
open source/external action
copy reference
request renderer/plugin action
```

Dynamic UI без fallback нарушает reviewability.

### Functional classes

Это не roadmap и не порядок реализации. Это функциональные классы, которые
должны помещаться в общую модель A-005.

#### Class A: Tool-rendered block

```text
ToolRenderedBlock:
  block_id
  surface_id
  tool_call_ref
  renderer_id
  input_snapshot
  output_snapshot optional
  status
  actions
  trace_refs
  fallback
```

States:

```text
proposed
running
output_available
output_error
cancelled
stale
archived
```

Tool-rendered block может показывать progress до завершения tool call. Он
должен уметь показывать arguments, status, result and error. Для sensitive
tool calls arguments/result могут быть redacted based on permissions.

#### Class B: Declarative dynamic block

```text
DeclarativeDynamicBlock:
  block_id
  surface_id
  schema_version
  component_catalog_id
  component_tree
  data_model
  bindings
  actions
  validation_rules
  trace_refs
  fallback
```

Подходит для:

- forms;
- tables;
- charts;
- dashboards;
- selectors;
- comparison views;
- lightweight wizards;
- generated reports.

Renderer использует Uprava/native component catalog. Agent supplies structure
and data, but host owns rendering behavior.

#### Class C: Generated UI artifact

```text
GeneratedUiArtifact:
  artifact_id
  artifact_type
  title
  description
  schema_or_package_ref
  data_model_ref
  renderer_contract
  version
  created_by_run_ref
  source_refs
  actions
  permissions
  fallback_snapshot
```

Generated UI artifact долговечнее block. Он может открываться в artifact
viewer, иметь detail view, обновляться, версионироваться, экспортироваться и
попадать в artifact gallery.

#### Class D: Sandboxed app artifact

```text
SandboxedAppArtifact:
  artifact_id
  package_ref
  entrypoint
  sandbox_policy
  csp_policy
  allowed_origins
  action_bridge_contract
  persisted_state_ref
  fallback_snapshot
  audit_refs
```

Sandboxed app может иметь executable code, но только внутри isolated runtime.
Main Uprava React tree не должен выполнять generated code.

### Renderer contract

Renderer contract связывает block/artifact type with actual UI implementation.

```text
RendererContract:
  renderer_id
  renderer_kind
  supported_block_types
  supported_schema_versions
  input_schema
  output_events
  supported_actions
  required_permissions
  trust_level
  fallback_strategy
```

Renderer kinds:

```text
core_renderer
plugin_renderer
declarative_schema_renderer
sandboxed_app_runtime
external_embed_runtime
fallback_renderer
```

Core renderer and trusted plugin renderer могут жить как обычные React
components. Declarative schema renderer рендерит approved component catalog.
Sandboxed runtime изолирует generated app. External embed должен быть rare and
explicitly permissioned.

### Component catalog

Component catalog - список approved building blocks, из которых agent может
собирать declarative UI.

Пример:

```text
ComponentCatalog:
  catalog_id: uprava.basic
  components:
    - text
    - heading
    - section
    - row
    - column
    - table
    - chart
    - form
    - input.text
    - input.number
    - input.select
    - input.checkbox
    - button.command
    - badge
    - code
    - file-ref
    - artifact-ref
```

Каждый component должен иметь:

```text
component type
props schema
allowed children
data binding rules
accessibility requirements
layout constraints
action rules
fallback behavior
```

Component catalog важен по трем причинам:

- агент получает bounded expressive language;
- Uprava сохраняет design-system consistency;
- clients can render the same artifact differently while preserving semantics.

### Dynamic UI proposal

Agent/tool/plugin может сначала создать proposal, а Core решает, как его
принять.

```text
DynamicUiProposal:
  proposal_id
  source
  target_surface
  proposed_kind
  renderer_or_catalog_id
  payload
  data_model optional
  actions optional
  refs
  requested_permissions
  fallback_payload
```

Proposal outcomes:

```text
accepted
accepted_with_transform
accepted_as_fallback_only
rejected_unsupported_type
rejected_permission_denied
rejected_invalid_schema
rejected_unsafe_payload
```

`accepted_with_transform` важно для случаев, где агент предложил слишком
богатый UI, а Core/Web может безопасно упростить его до table, markdown or
static snapshot.

### Lifecycle: tool-rendered block

```text
Tool registered in Tool Registry with renderer contract
-> agent calls tool
-> Core records ToolCallStarted event
-> Web shows running block through renderer or fallback
-> Node/plugin/tool returns result stream or final result
-> Core validates output and updates block/artifact metadata
-> Web renders result state
-> user invokes action
-> Core checks permission and dispatches command
-> action result updates block, artifact or trace
```

Important events:

```text
tool_call.started
tool_call.progress
tool_call.output_available
tool_call.output_error
dynamic_block.created
dynamic_block.updated
dynamic_block.action_requested
dynamic_block.action_completed
dynamic_block.action_failed
```

### Lifecycle: generated UI artifact

```text
agent proposes generated artifact
-> Core validates proposal
-> Core creates artifact metadata and initial version
-> Web opens artifact block/viewer
-> user interacts locally or through action bridge
-> important changes become events
-> agent/tool may update artifact data model
-> Core creates new artifact version or state update
-> fallback snapshot updated when needed
```

Important events:

```text
dynamic_ui.proposed
dynamic_ui.accepted
dynamic_ui.rejected
artifact.created
artifact.version_created
artifact.state_updated
artifact.action_requested
artifact.action_completed
artifact.fallback_snapshot_updated
```

### Action bridge

Dynamic UI needs a narrow bridge from UI interaction to Core actions.

```text
ActionBridge:
  action_id
  action_kind
  label
  input_schema
  target
  required_permissions
  confirmation_policy
  idempotency_key optional
```

Action kinds:

```text
submit_form
update_artifact_state
invoke_command
send_agent_input
open_reference
create_artifact
export_artifact
request_approval
```

Bridge rules:

- every action has stable `action_id`;
- action payload is schema-validated;
- privileged actions require Core permission checks;
- destructive/open-world actions require explicit confirmation policy;
- actions are logged as events;
- sandboxed UI can only use allowed bridge actions;
- action result must be reflected back into UI state or trace.

### State and persistence

Dynamic UI state has several layers:

```text
descriptor state
data model state
local interaction state
artifact version state
execution/sandbox state
projection state
```

Only some state is worth persisting. The rule:

```text
If losing state changes review, decision, result, reproducibility or future
agent context, persist it outside the frontend renderer.
```

Examples:

- selected tab in a dashboard may be local;
- edited assumptions in a calculator should be persisted;
- submitted form values must be event-backed;
- expanded/collapsed rows usually can remain local;
- generated artifact data snapshot must be persisted;
- sandbox internal transient animation state should not matter.

Generated UI artifact should support versions:

```text
artifact version 1: initial generated schema/data
artifact version 2: user edited assumptions
artifact version 3: agent refreshed data
artifact version 4: exported/reviewed state
```

Versioning does not mean every UI click creates a version. It means
meaningful artifact state transitions are durable and reviewable.

### Permissions and trust levels

Trust levels:

```text
core renderer
trusted bundled plugin renderer
installed local/team plugin renderer
declarative generated UI
sandboxed generated app
external embed
fallback only
```

Permission dimensions:

```text
can_render
can_read_data
can_interact
can_update_artifact_state
can_invoke_commands
can_send_agent_input
can_access_external_network
can_open_external_urls
can_use_files
can_persist_state
```

Dynamic UI must not imply automatic data access. A generated dashboard might be
allowed to render aggregate metrics but not raw logs. A form might be visible
but disabled for a user without submit permission. A sandboxed app might run
local calculations but cannot fetch external resources.

### External systems

External embeds should not be the default form of dynamic UI.

Preferred ladder:

```text
external link
-> rich preview
-> artifact snapshot
-> controlled sandboxed embed
```

For example, a Grafana link should usually become:

```text
Grafana dashboard reference
-> Uprava preview block
-> incident/status artifact
-> trace refs
-> open external action
```

Full embed is justified only if it materially reduces context switching and
can be governed through permissions, origin allowlists, CSP and fallback.

### Relationship with Tool Registry and Plugin Registry

Tool Registry should know:

```text
tool input/output schemas
tool permissions
tool execution location
tool annotations
default renderer contract
action mappings
audit policy
```

Plugin Registry should know:

```text
provided renderers
component catalogs
artifact types
sandbox runtimes
external origins
commands/actions
permissions
compatibility
trust level
```

Dynamic UI is where Tool Registry, Plugin Registry and A-004 work surface meet.
If a tool can create visual output, that visual output should be registered and
traceable. If a plugin adds a generated artifact type, it should register
renderer, fallback and permission model.

Первый A-005 slice должен одновременно доказать полезный bundled dynamic UI
plugin и расширить общую plugin platform: versioned component-catalog and
dynamic-renderer contributions, permissioned action bridge, configuration/
context keys, isolation and disable/failure fallback. Ни один из этих contracts
не должен быть приватным API только для bundled package.

### Relationship with A-006 Visual Rendering and Artifact Semantics

A-005 and A-006 are related but not identical.

```text
A-005:
  how agents can create dynamic UI blocks/artifacts

A-006:
  how visual objects behave across inline renderers, viewers, blocks,
  artifact viewers and external previews
```

A chart can be:

- inline/viewer visual object described by A-006 semantics;
- tool-rendered block from A-005;
- component inside declarative generated dashboard;
- widget inside sandboxed app artifact.

The visual semantics belong to A-006: source-of-truth, render scope,
addressability, fallback, actions, cause refs and artifact promotion.
The agent-generated lifecycle, permissions and dynamic mounting belong to
A-005.

If an agent writes Mermaid in Markdown, that is usually A-006 inline rendering,
not A-005 dynamic UI. If an agent explicitly proposes a generated dashboard
artifact, creation belongs to A-005 and visual semantics belong to A-006.

### Relationship with A-008 Go to Source and Causality UX

Every meaningful dynamic UI object should expose cause refs:

```text
block -> tool call
chart -> query/data source
form -> agent question/context
dashboard cell -> source event/check/artifact
calculator output -> assumptions/version/formula
button -> command/action
```

Go to Source / Cause should work from dynamic UI the same way it works from
diff, terminal output or artifacts. Dynamic UI must not hide source/evidence
and causality behind visual polish.

### Relationship with A-009 Human-Agent Dual Interface

Dynamic UI is a major input to human-agent dual interface.

For humans, it provides visual interaction.

For agents, it should provide structured UI state:

```text
current surface
visible blocks
selected object
component tree
data model
validation errors
available actions
permissions
trace refs
artifact state
```

This lets an internal Uprava agent answer questions like:

- "Что я сейчас вижу?"
- "Почему эта кнопка disabled?"
- "Какие данные стоят за этим chart?"
- "Какие actions доступны из этого dashboard?"
- "Что изменилось с прошлой версии artifact?"

### Failure modes

| Failure | Expected behavior |
| --- | --- |
| Unknown renderer | Show fallback metadata and sanitized payload. |
| Invalid schema | Reject proposal, record error, show agent-readable validation feedback. |
| Permission denied | Show disabled/read-only UI or fallback with reason. |
| Renderer plugin disabled | Show fallback and optional enable/install action if allowed. |
| Sandbox package unsafe | Reject executable runtime, possibly accept static snapshot. |
| External origin not allowed | Block embed, show external link only if allowed. |
| Action failed | Record action failure event and update UI state. |
| Artifact state too large | Require artifact storage reference or summarized snapshot. |
| Mobile/client unsupported | Render simplified fallback or compatible component subset. |
| Agent generated misleading UI | Preserve source refs and allow review/go-to-cause. |

### Quality questions

Dynamic UI should be evaluated by product and architecture questions:

- Does this UI reduce review cost compared with text?
- Is it backed by artifact/event metadata?
- Can the same result be read without the rich renderer?
- Are actions permissioned and traceable?
- Can the agent understand the UI state it created?
- Can the user go from visible result to cause?
- Is executable code avoided unless declarative schema is insufficient?
- If executable code is used, is it sandboxed and capability-scoped?
- Does the UI fit known Uprava surfaces instead of taking over the workbench?
- Can it survive reconnect, reload, plugin disable and missing renderer?

## Reference patterns

Several external patterns inform this direction:

- Vercel AI SDK style generative UI: model tool calls mapped to React
  components.
- CopilotKit tool rendering/components-as-tools: frontend components exposed
  as typed tools and backend tool calls rendered with custom UI.
- Adaptive Cards: declarative JSON UI rendered natively by host application.
- A2UI: agent-to-UI declarative surfaces with component catalog and actions.
- OpenAI Apps SDK / MCP Apps: tool results connected to sandboxed UI resources.
- Anthropic Artifacts: durable creative/app-like space for code, documents,
  visualizations and interactive objects.
- VS Code Webviews: powerful but isolated custom UI surfaces with CSP,
  lifecycle and state concerns.

Uprava should not copy any one model directly. The product needs a hybrid:

```text
tool-rendered blocks for traceable agent/tool work
+ declarative generated UI for forms/dashboards
+ artifact-backed generated app surfaces when necessary
+ strict Core-level permissions and fallback everywhere
```

## Рабочая формула

Dynamic UI from Agents is the Uprava mechanism for turning agent/tool/plugin
work into traceable, permissioned, reviewable interactive blocks and artifacts.

It is built on A-004 surfaces, blocks, artifacts, references, commands and
renderers. It adds a dynamic creation path:

```text
tool-rendered blocks
declarative generated UI
generated UI artifacts
sandboxed app artifacts
```

Агент может выбирать, создавать, параметризовать and обновлять UI через
approved contracts. Core validates and stores it. Web renders it through known
renderers, declarative component catalogs or sandbox runtimes. User actions
return as commands/events. Every important UI object has trace refs and a safe
fallback.

Первая поставка этой механики является bundled first-party plugin. Последующие
plugins могут добавлять новые form/dashboard families через те же versioned
contributions, не изменяя App Shell и не обходя Core authorization.

Главная граница: dynamic UI должен увеличивать способность пользователя
понимать, проверять and управлять агентской работой, не превращая Uprava в
неуправляемую среду произвольного generated code.
