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
предоставляет opt-in Generated React runtime, Uprava React SDK, design
tokens, layout contract, renderers and related contributions через общий
Plugin Registry/Extension Host contract. Generated React исполняется в
sandboxed artifact runtime, а не подставляется в main workbench React tree.
По мере развития A-005 этот contract должен становиться пригодным для
local/team/community plugins, как в extension ecosystems Obsidian и VS Code.

Документ намеренно не описывает, что попадет в первую или вторую версию
продукта. Scope конкретных итераций должен определяться отдельно. Здесь
проектируется целое направление: какие формы dynamic UI нужны Uprava, как они
связаны с agent work, где проходит граница безопасности, и какие базовые
архитектурные контракты нельзя сломать будущими реализациями.

## Зафиксированное решение

Основной expressive model — generated React/TypeScript, а не закрытый язык
разрешенных UI-блоков. Каталог готовых components сохраняется как
Uprava React SDK and optional declarative fast path, но не ограничивает
выразительность generated UI.

Эта возможность выключена по умолчанию и включается явно. Даже после
включения generated code никогда не монтируется в main React tree:
он становится durable artifact, собирается controlled pipeline и
исполняется в sandboxed iframe с capability-scoped bridge. Настройка дает
consent, но не отменяет изоляцию.

По layout принят artifact-first approach: компактные forms/cards могут
жить inline, а сложные dashboards, graphs, calculators and editors открываются
в отдельной canvas surface; timeline показывает их preview. Host владеет
artifact frame, bounds, scroll and transition to fullscreen.

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

Dynamic UI в Uprava может быть generated React, но это не
"произвольный React в основном tree чата". React/TypeScript package является
версионируемым artifact, проходит validation/build и запускается в
изолированном runtime.

Базовая модель:

```text
agent/tool/plugin proposes UI intent or Generated React artifact
-> Core validates type, schema, refs, permissions and trace metadata
-> Core stores block/artifact/event metadata
-> controlled build validates and bundles the artifact when code is present
-> Web mounts the UI through an approved sandboxed renderer/runtime
-> user interaction becomes command/action/event
-> Core routes action through registry and permissions
-> result updates artifact, block, trace or workflow state
```

Агент может:

- выбрать подходящий зарегистрированный visual block;
- заполнить typed props/data для renderer-а;
- создать Generated React/TypeScript artifact;
- предложить declarative UI schema как optional fast path;
- обновлять data model or artifact snapshot;
- запросить user action через форму, control, approval или command;
- связать UI с trace refs, files, commands, tool calls and artifacts.

Агент не должен автоматически получать право:

- выполнять произвольный JavaScript в основном React tree;
- выходить за границы artifact frame или управлять workbench shell;
- обходить Core permissions;
- вызывать privileged commands напрямую из generated UI;
- скрывать external API calls за визуальным элементом;
- хранить важное состояние только внутри ephemeral frontend component;
- создавать UI, который невозможно прочитать как artifact/fallback.

Короткая формула:

```text
Agent can generate and parameterize UI artifacts.
Core decides whether it is allowed.
Web renders them in a bounded sandbox through known contracts.
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

#### Generated React artifact

Generated React artifact — основной expressive path для forms, dashboards,
charts, tables, reports, calculators, simulations and custom interactive tools.
Агент создает React/TypeScript package под versioned Uprava React SDK. Package
хранится как artifact, проходит controlled build/validation pipeline и
исполняется в sandboxed iframe or equivalent isolated runtime.

Пакет может описывать:

```text
React/TypeScript source
entrypoint and dependency lock
SDK/API version
data and persisted state schema
layout intent
requested capabilities and actions
fallback snapshot/representation
source and trace refs
```

Generated code может использовать обычные React components, local state,
HTML and scoped styles. Uprava SDK дает готовые components, design tokens,
artifact state, source refs and action bridge, но не является закрытым
языком разрешенных блоков.

Свойства:

- arbitrary generated code does not execute in the main workbench tree;
- strict CSP, capability policy and message bridge;
- artifact-owned identity, source, state, versions and fallback;
- preferred consistency through Uprava SDK without expressive ceiling;
- full fidelity on capable web clients and safe fallback elsewhere;
- machine-readable metadata/state alongside the visual runtime.

Для Uprava это должен быть основной путь для generated UI.

#### Optional schema-driven UI

Schema-driven UI может остаться как опциональный fast path для маленьких
forms, cards, approvals and simple reports. Агент в этом случае заполняет
declarative component tree and data model, а host рендерит его без
generated code.

Это полезная оптимизация, но не главная architectural foundation. Uprava
не должна выращивать собственный declarative frontend language для всех
будущих interaction patterns. Generated React используется, когда простого
catalog недостаточно.

Оба path живут в общей artifact model:

```text
simple known interaction -> optional declarative fast path
general generated UI      -> React/TypeScript artifact in sandbox
```

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
  - runtime_id: uprava.generated-react
    kind: generated-react-artifact
    sdk_version: ...
    allowed_dependencies: ...
    allowed_layouts: [inline, panel, canvas]
    available_capabilities: ...
  - renderer_id: basic-form
    kind: optional-schema-driven-ui
    component_catalog: uprava.basic
```

Агент не должен угадывать frontend internals. Он должен работать с
capabilities:

- "можно показать chart";
- "можно создать form";
- "можно открыть generated dashboard artifact";
- "можно создать React artifact для canvas";
- "можно запросить approval";
- "можно обновить существующий artifact";
- "generated UI выключен или нужна запрещенная capability".

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
Generated UI Runtime
Uprava React SDK
Layout Intent
Component Catalog optional
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
but not submittable, generated runtime может делать local calculations but
cannot call external network.

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

#### Class B: Optional declarative dynamic block

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

- small forms and approvals;
- cards and selectors;
- simple tables;
- lightweight wizards and reports.

Renderer использует Uprava/native component catalog. Agent supplies structure
and data, but host owns rendering behavior. Это fast path для простых
блоков, а не ограничение общей generated UI модели.

#### Class C: Generated React artifact

```text
GeneratedReactArtifact:
  artifact_id
  artifact_type
  title
  description
  source_package_ref
  compiled_bundle_ref
  dependency_lock_ref
  sdk_version
  runtime_id
  data_model_ref
  persisted_state_ref
  layout_intent
  requested_capabilities
  version
  created_by_run_ref
  source_refs
  actions
  permissions
  fallback_snapshot
```

Generated React artifact долговечнее block. Он может открываться в artifact
viewer, иметь detail view, обновляться, версионироваться, экспортироваться и
попадать в artifact gallery.

#### Class D: Sandboxed generated UI runtime

```text
GeneratedUiRuntime:
  runtime_id
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

Sandboxed runtime исполняет generated code в iframe or equivalent isolated
environment. Main Uprava React tree не должен выполнять generated code даже
для bundled plugin или trusted-local mode.

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
generated_react_runtime
external_embed_runtime
fallback_renderer
```

Core renderer and trusted plugin renderer могут жить как обычные React
components. Но generated React всегда изолируется runtime-ом. Declarative schema
renderer рендерит optional component catalog. External embed должен быть rare
and explicitly permissioned.

### Uprava React SDK and optional component catalog

Uprava React SDK — основной authoring contract для generated UI. Он дает
artifact shell primitives, design tokens, responsive layout, forms, tables,
charts, source refs, persisted state and permissioned actions. Agent может
собирать UI из готовых components или писать собственные components внутри
sandbox. Component catalog может дополнительно описывать approved building
blocks для optional declarative fast path.

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

SDK and catalog важны по трем причинам:

- агент получает стабильные primitives без закрытого expressive ceiling;
- Uprava сохраняет design-system consistency;
- clients can render the same artifact differently while preserving semantics.

### Generated React runtime and build

Generated UI выключен по умолчанию и включается operator/user policy на
разрешенном deployment, project or workspace scope. Включение является
согласием на эту функцию, но не security boundary. Изоляция, CSP,
capability checks and Core authorization сохраняются во всех режимах.

Начальные modes:

```text
off
  generated UI не build-ится и не исполняется; доступен fallback

sandboxed
  generated React исполняется с минимальными granted capabilities

trusted_local later
  operator может дать более широкие capabilities, но runtime остается
  изолированным от main tree
```

Controlled build pipeline должен фиксировать React/runtime/SDK versions,
dependency lock, diagnostics and output bundle hash. Первый slice не должен
разрешать произвольный `npm install`: runtime предоставляет React, Uprava
SDK and a small versioned allowlist of libraries. Это сохраняет reproducibility и
ограничивает supply-chain surface.

### Layout contract

Workbench shell and artifact chrome принадлежат Uprava. Generated UI управляет
только body внутри artifact frame и не может менять global navigation,
router, overlays, keyboard shortcuts или CSS host-а.

Artifact объявляет layout intent:

```text
inline
  compact form, approval or tool result в timeline; bounded height

panel
  Inspector/sidebar-compatible view с минимальной шириной

canvas
  основная surface для dashboard, graph, calculator or editor

fullscreen
  только по явному user action; artifact не активирует его сам
```

Базовое product rule: маленькие forms/cards могут жить inline; UI сложнее
простой таблицы преимущественно открывается в canvas, а timeline показывает
preview card.

Layout rules:

- generated UI ориентируется на container size, а не на global viewport;
- SDK предоставляет container queries, design tokens and resize hooks;
- iframe не может раздвинуть parent выше host-owned bounds;
- inline frame имеет bounded height and explicit `open in canvas` action;
- в каждом mode должен быть один понятный owner вертикального scroll;
- таблицы и большие lists используют controlled overflow/virtualization;
- styles ограничены iframe; external fonts, images and network требуют
  отдельных capabilities.

Runtime может посылать host-у bounded lifecycle/layout messages such as
`ui.ready`, `ui.preferred_size_changed`, `ui.content_overflow` and
`ui.request_open_canvas`. Host валидирует и ограничивает любой requested size.

### Dynamic UI proposal

Agent/tool/plugin может сначала создать proposal, а Core решает, как его
принять.

```text
DynamicUiProposal:
  proposal_id
  source
  target_surface
  proposed_kind
  runtime_or_renderer_id
  source_package_ref optional
  sdk_version optional
  layout_intent optional
  payload optional
  data_model optional
  actions optional
  refs
  requested_capabilities
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
rejected_build_policy
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
-> controlled builder validates imports, locks versions and produces bundle
-> Core records build diagnostics, bundle hash and runtime policy
-> Web opens fallback/preview card or sandboxed artifact viewer
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
artifact.build_started
artifact.build_completed
artifact.build_failed
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
optional declarative generated UI
sandboxed Generated React artifact
trusted-local Generated React artifact
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
can_request_layout_change
```

Dynamic UI must not imply automatic data access. A generated dashboard might be
allowed to render aggregate metrics but not raw logs. A form might be visible
but disabled for a user without submit permission. A generated React artifact
might run local calculations but cannot fetch external resources.

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
generated UI runtimes
React SDK/API versions
allowed dependency sets
layout contracts
component catalogs optional
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

Если несколько plugins предоставляют contributions для одного dynamic artifact
target, выбор и порядок определяет общий contract
[`A-012 Plugin Contribution Resolution`](012-plugin-contribution-resolution.md).
Generated React runtime сохраняет explicit sandbox boundary: host-level order
может выбирать runtime, shell contributions and actions, но не компонует
произвольные plugin React trees внутри iframe.

Первый A-005 slice должен одновременно доказать полезный bundled dynamic UI
plugin и расширить общую plugin platform: versioned Generated React runtime,
Uprava React SDK, layout/dynamic-renderer contributions, permissioned action
bridge, configuration/context keys, isolation and disable/failure fallback.
Ни один из этих contracts не должен быть приватным API только для bundled
package.

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
- component inside optional declarative dashboard;
- widget inside sandboxed Generated React artifact.

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
| Build or dependency policy failed | Preserve source package and diagnostics; show fallback without executing the bundle. |
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
- Is generated code sandboxed and capability-scoped regardless of trust mode?
- Is a simple declarative/tool-rendered block preferable for this small case?
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
+ optional declarative fast path for small known interactions
+ Generated React artifacts for expressive forms/dashboards/apps
+ sandboxed execution with an Uprava React SDK and layout contract
+ strict Core-level permissions and fallback everywhere
```

## Рабочая формула

Dynamic UI from Agents is the Uprava mechanism for turning agent/tool/plugin
work into traceable, permissioned, reviewable interactive blocks and artifacts.

It is built on A-004 surfaces, blocks, artifacts, references, commands and
renderers. It adds a dynamic creation path:

```text
tool-rendered blocks
optional declarative dynamic blocks
Generated React artifacts
sandboxed generated UI runtimes
```

Агент может выбирать, создавать, параметризовать and обновлять UI через
approved contracts. Core validates and stores it. Controlled builder фиксирует
runtime/SDK/dependency versions. Web исполняет generated React в sandbox, а не
в main tree. User actions return as commands/events. Every important UI
object has trace refs, persisted review-relevant state and a safe fallback.

Первая поставка этой механики является выключенным по умолчанию bundled
first-party Generated React plugin. Последующие plugins могут добавлять новые
generated UI runtimes and families через те же versioned contributions, не
изменяя App Shell и не обходя Core authorization.

Главная граница: dynamic UI должен увеличивать способность пользователя
понимать, проверять and управлять агентской работой, не превращая Uprava в
неуправляемую среду произвольного generated code.
