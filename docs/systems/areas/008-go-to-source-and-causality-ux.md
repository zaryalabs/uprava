# A-008 Go to Source and Causality UX

Статус: `implemented-0.2.7; target-direction-active`

Этот документ фиксирует корневую позицию по ключевой механике `A-008 Go to
Source and Causality UX`.

## Реализованный baseline 0.2.7

Первый useful slice реализует честную coarse causality поверх существующего
event log:

- `SessionTraceProjection` группирует messages, runtime/provider activity,
  commands и workspace observations в steps с `exact`, `coarse` или `unknown`
  precision;
- Core API предоставляет cursor-based `/events`, raw event detail и
  `/references/resolve` с явными состояниями `resolved`, `missing`, `offline`,
  `redacted`, `unsupported` и `raw_only`;
- Node создаёт типизированные события для file write, workspace command,
  check и diff observation с source/evidence/cause/result refs;
- Web Session surface показывает causal steps, фильтруемый raw event log и
  aspect-based Context Inspector;
- Deduction является отдельной командой, а не продолжением live transcript:
  Node запускает Codex как `ephemeral` read-only process со structured output
  schema, Core ограничивает evidence package, проверяет provenance и allowlist
  refs, сохраняет invalid raw fallback и поддерживает cancellation;
- валидный `DeductionBlock` остаётся transient, пока пользователь явно не
  сохранит его как versioned `CausalityNarrative`.

Baseline не заявляет exact per-edit provenance, полный cause graph или доступ к
внутреннему reasoning provider. Такие пробелы остаются явными через precision и
raw fallback.

Follow-up baseline `0.2.10` делает review evidence адресуемым: workspace Review
сохраняет bounded diff snapshot, публикует `WorkspaceDiff` и стабильные
`DiffHunk` refs, а controlled checks публикуют `CheckResult` с причиной
`Command`. Diff остаётся snapshot-level evidence и не выдаётся за точную
причину конкретного edit.

Главная позиция: `A-008` - это не отдельный trace viewer и не попытка показать
пользователю весь raw log. Это **механика перехода от видимого результата к
источнику, доказательству и причине** через ссылки между addressable blocks,
events, artifacts, files, commands and agent steps.

Короткая формула:

```text
Visible result
-> source: из чего это отрендерено или процитировано
-> evidence: чем это подтверждается
-> cause: какое действие/решение/контекст это породило
```

На раннем этапе детализация может быть грубой: ответ агента, turn, tool call,
terminal command, output segment, diff hunk, artifact or log text. Позже та же
модель должна углубляться до строк, диапазонов, chart points, visual object
parts and structured cause graph.

## Vision

### Какую проблему решает механика

Пользователь видит ответ агента, diff, check failure, artifact, dashboard или
UI block, но часто не понимает:

- откуда взялся конкретный вывод;
- что агент реально сделал, а что только написал;
- какая команда, tool call, файл, лог или внешний источник лежит за
  утверждением;
- почему появился diff hunk, failed check or artifact state;
- можно ли доверять шагу или он является agent-authored summary;
- где raw evidence, если rich UI ошибся или недоступен.

Обычный trace log решает это плохо. Если показать весь stream действий, review
становится дороже: пользователь вынужден читать шумные события, повторяющиеся
логи, промежуточные рассуждения, tool output and transport details.

`A-008` должен дать UX, похожий по роли на `go to definition`, но для
агентской работы:

```text
I see a result.
I open the source/cause only when I need it.
I can keep drilling down until raw evidence or original request.
```

Цель не в том, чтобы доказать каждое слово формально. Цель - снизить стоимость
понимания, review, handoff, return and accountability.

### Source, evidence and cause

Нужно не смешивать три типа связи.

#### Source

`Source` отвечает на вопрос: **из чего это отрендерено, процитировано или
получено как данные?**

Примеры:

```text
answer paragraph -> session message range
Mermaid diagram -> Markdown code fence range
test report row -> test output range
chart point -> query result row
file preview -> file range
external preview -> external entity snapshot
artifact view -> artifact version snapshot
```

Source обычно нужен для `open source`, fallback, copy reference and editing
semantics.

#### Evidence

`Evidence` отвечает на вопрос: **какое наблюдаемое подтверждение поддерживает
это утверждение или состояние?**

Примеры:

```text
"tests pass" -> check result event + command output
"file changed" -> diff hunk + file write event
"issue exists" -> external snapshot + fetch event
"agent read config" -> file read event + source range if known
```

Evidence может не быть причиной. Это может быть проверка результата, внешний
snapshot или подтверждающий лог.

#### Cause

`Cause` отвечает на вопрос: **какое действие, решение или контекст породили
этот результат?**

Примеры:

```text
diff hunk -> file edit event -> agent step -> user request
failed check -> command event -> tool invocation -> agent decision
artifact update -> dynamic UI action -> command -> previous artifact state
agent answer -> tool results + context refs + user prompt
```

Cause links должны быть направленными и typed. Не всякая related link является
causality.

### Уточнение реализации: trace, evidence и deduction

`Trace` — последовательность наблюдаемых событий. `Evidence` — конкретный
проверяемый объект. Ни одно из них не обязано восстанавливать внутреннее
рассуждение агента или доказывать полную причинность.

```text
user message
-> assistant turn
-> command / workspace observation
-> workspace changed
-> check
-> assistant answer
```

Источники доверия нужно показывать явно: system/node event сильнее provider
event, а provider event сильнее agent-authored claim.

`Deduction` — отдельный запрошенный пользователем режим объяснения, например
`/deduction <scope or question>`, а не автоматически восстановленная
causality. Core формирует специальный запрос к provider с требованием вернуть
валидный structured result. Такой результат является agent-authored
interpretation поверх доступного контекста, а не source of truth.

Каждый вывод Deduction должен различать:

- `observed` — подтверждено refs или evidence;
- `inference` — вывод агента;
- `assumption` — предположение;
- `unknown` — известный пробел;
- `alternative` — другая правдоподобная интерпретация.

Минимальный `DeductionBlock` содержит `scope_ref`, `conclusion`, `steps`,
`support_refs`, `certainty`, `assumptions`, `unknowns` и `alternatives`.
Невалидный provider result не становится rich block: UI показывает text/raw
fallback с причиной validation failure.

### Session trace projection и честная точность

Общая session-wide read model называется `SessionTraceProjection`. Она связывает
turns, messages, commands, workspace observations, checks и object refs.

```text
TraceObservation:
  ref
  scope_ref
  turn_ref
  happened_at
  kind
  label
  object_refs
  evidence_refs
  precision: exact | turn_level | snapshot
```

`precision` обязателен: UI не должен выдавать snapshot или turn-level signal за
точный edit. Текущий `WorkspaceDiffResponse` — diff состояния workspace по
запросу, а не diff каждого изменения. Поэтому первый срез честно показывает
`turn -> workspace changed -> current workspace diff`; exact per-edit diff
требует before/after snapshots, filesystem audit, VCS checkpoints или
provider-normalized edit events.

Один inspector/projection обслуживает два UI scope:

- **Turn Activity** — действия и evidence одного assistant turn;
- **Chat Trace** — история выбранного file, command, check, artifact или
  message через всю сессию.

### Live projection contract

Session event stream является realtime read-model input, а не сигналом
перезагрузить все projections. Contiguous event сразу добавляется в conversation
и открытые trace/evidence/raw views, патчит session summary внутри inventory и
обновляет известные runtime поля Agent Projection. Поэтому network coalescing
не снижает визуальную частоту обновлений.

Canonical Core snapshots остаются обязательными для initial load, reconnect,
sequence gap и полей, которые нельзя честно вывести из одного event envelope.
Например, полный набор `available_commands` перечитывается на lifecycle,
approval и workspace boundaries, но `provider.activity` не запускает такой
refetch. При message/approval/error boundary Trace и Evidence после мгновенного
локального update сверяются с canonical projection, чтобы заменить временные
client refs durable Core identifiers.

### Главная модель

Любой важный видимый объект Uprava должен иметь stable reference and optional
links:

```text
VisibleObject:
  ref
  source_refs
  evidence_refs
  cause_refs
  related_refs
  available_actions
  fallback
```

Reference открывается не обязательно как новая страница. В desktop UI это
скорее inspector, drawer, popover, peek window or stacked detail panel. В
mobile UI та же модель превращается в navigation stack.

```text
Answer block
  link: "steps"
    -> Agent steps detail
      -> Tool call
        -> Command output
          -> Raw log segment
      -> File edit
        -> Diff hunk
          -> File range
```

Блоки могут выступать как popups/detail views. Это не отдельная система
модальных окон, а использование модели из `A-004`: reference -> detail view ->
aspects -> deeper references.

### Почему это отдельное направление

`A-004` уже говорит, что UI должен быть addressable и иметь links/detail views.
`A-005` говорит, что агент может создавать dynamic blocks/artifacts. `A-006`
говорит, что visual objects должны знать source-of-truth, fallback and cause
refs.

`A-008` выделяется отдельно, потому что здесь другой вопрос:

```text
Как пользователь двигается по цепочке "результат -> источник -> причина",
не превращая UI в raw trace dump?
```

Без отдельной модели будет два плохих исхода:

- ссылки останутся случайными `open raw log` ссылками без продуктовой
  семантики;
- или Uprava построит огромный trace graph, который теоретически точный, но
  практически нечитаемый.

Рабочая позиция: начать с маленького, coarse-grained source/cause graph,
который хорошо работает в UI, и расширять гранулярность только там, где она
реально снижает review cost.

### Пользовательские сценарии

#### 1. Переход от ответа к шагам агента

Пользователь читает ответ агента:

```text
Я обновил конфигурацию, добавил тест и проверил make c.
```

В ответе есть action/link:

```text
Open steps
```

Uprava показывает detail block:

```text
Steps
1. Read config files
2. Edited docs/...
3. Ran make c
4. Summarized result
```

На раннем этапе каждый шаг может быть просто текстовым summary с link to raw
event/log/message. Если есть richer renderer, шаг может раскрыться как command
block, file diff, test report or artifact preview.

#### 2. Переход от diff hunk к причине

Пользователь смотрит diff и выбирает hunk:

```text
go to source/cause
```

Uprava раскрывает:

```text
Diff hunk
-> file edit event
-> agent step "update runtime model"
-> source context: user request + design doc section
-> related command/check
```

Если точный cause неизвестен, UI не должен притворяться:

```text
Cause: unknown
Evidence: file write event, runtime session, nearby agent message
Raw trace available
```

#### 3. Переход от failed check к команде и логу

Пользователь видит failed check block:

```text
make c failed
```

Drilldown:

```text
Check result
-> terminal command
-> output range with failure
-> related files if parser knows them
-> agent action that invoked the command
```

Если parser может извлечь file/line из output, это становится source/evidence
ref. Если не может, fallback - raw output range.

#### 4. Переход от dynamic UI к исходным данным

Dashboard cell, chart point, form validation error or generated report section
должны раскрывать source/cause так же, как обычный текст:

```text
chart point
-> source data row/query result
-> tool call that fetched data
-> external snapshot
-> agent step that created dashboard
```

Dynamic UI не должен скрывать причинность за визуальной полировкой.

#### 5. Запрошенный Deduction

Пользователь выбирает `Explain steps` или задаёт `/deduction <scope or
question>`. Core добавляет в provider request доступные события и refs, но
требует отдельный JSON result, а не свободный narrative в обычном ответе.

```text
deduction.requested
-> provider structured result
-> validation
-> deduction.completed | deduction.invalid
-> DeductionBlock or text/raw fallback
```

`DeductionBlock` рендерится как linked block/artifact. Он группирует
низкоуровневые events для review, но не подменяет их: каждый существенный step
содержит support refs либо явно отмечает uncertainty. Optional tool наподобие
`uprava.emit_causality_narrative` остаётся способом для agent/tool создать
сохранённый `CausalityNarrative`, но пользовательский Deduction не зависит от
этого tool path.

### Agent-facing сценарии

Для internal Uprava agent and provider adapters `A-008` дает структурированный
язык:

- объяснить выбранный UI object;
- собрать review summary с refs;
- ответить "почему это изменилось?";
- найти source для assertion/diff/check/artifact;
- построить handoff note по trace;
- показать недостающие evidence links;
- предложить улучшение parser-а или tool renderer-а, если raw logs слишком
  часто остаются единственным fallback.

Агент не должен полагаться только на screenshot или HTML. Ему нужен
machine-readable state:

```text
selected_ref
visible_refs
source/evidence/cause refs
available actions
permission status
raw fallback availability
```

## Architecture

### Основные сущности

#### UpravaRef

Stable address на сущность, диапазон или визуальный объект.

```text
UpravaRef:
  kind
  id
  version optional
  range optional
  parent_ref optional
  permission_scope
```

Примеры `kind`:

```text
session
turn
message
message_range
agent_step
tool_call
terminal_command
terminal_output_range
file
file_range
diff_hunk
check_result
artifact
artifact_version
visual_object
external_entity
external_snapshot
raw_event
```

#### Link

Typed directed relation между refs.

```text
Link:
  from_ref
  to_ref
  relation
  provenance
  confidence optional
  created_by
```

Базовые relation types:

```text
rendered_from
quoted_from
produced_by
verified_by
caused_by
requested_by
used_context
invoked
emitted
changed
checked_by
explains
summarizes
related_to
supersedes
```

`provenance` важен. Ссылка, созданная Core из command event, надежнее, чем
ссылка, которую агент вывел из текста. Agent-authored links допустимы, но UI
должен уметь отличить их от system-derived links.

#### StepBlock

Review-facing grouping низкоуровневых events.

```text
StepBlock:
  ref
  title
  summary
  time_range
  actor
  source_refs
  evidence_refs
  cause_refs
  result_refs
  child_refs
  raw_event_refs
```

StepBlock может быть создан:

- provider adapter parser-ом;
- Core event aggregation;
- tool renderer-ом;
- agent-authored causality narrative;
- human annotation.

#### DeductionBlock и CausalityNarrative

`DeductionBlock` — валидированный ответ на явный deduction request. Он может
быть transient block или сохранён как `CausalityNarrative` artifact, если нужен
для review/handoff. `CausalityNarrative` также может быть создан agent/tool, но
всегда сохраняет provenance и не становится system-derived fact.

```text
DeductionBlock:
  ref
  scope_ref
  title
  conclusion
  steps
  support_refs
  certainty
  assumptions
  unknowns
  alternatives
```

Если narrative влияет на review/handoff, его стоит сохранять как artifact
version. Если это quick popup explanation, он может быть transient view over
events.

### Границы ответственности

Core owns:

- stable refs;
- event log and trace metadata;
- `SessionTraceProjection` and its declared precision;
- system-derived links;
- permissions for resolving refs;
- deduction request/result validation and fallback state;
- storage of narrative artifacts;
- command routing for source/cause actions.

Node Daemon owns:

- local file, terminal, process and command evidence;
- raw output ranges when data is local/large;
- parsers close to workspace tools when practical;
- workspace boundary enforcement.

Provider adapter owns:

- normalization of provider events;
- coarse parsing of agent actions, tool calls, approvals and output;
- mapping provider-specific ids to Uprava refs when possible.

Web owns:

- rendering links, popovers, detail views and drilldown stack;
- source/cause action presentation;
- safe fallback for unresolved refs;
- navigation state and selected object;
- visual distinction between source, evidence, cause and related links.

Agent/tool owns:

- optional structured narratives and the interpretation inside a Deduction;
- semantic grouping of steps;
- explicit uncertainty when links are incomplete;
- no invention of unreferenced facts.

### UI consequences

Every important block/detail view should expose a small common action set:

```text
open source
open cause
open evidence
open related
explain steps
copy reference
open raw event
```

Not every action appears everywhere. The UI should show what exists for the
current object and permissions.

Detail views should be aspect-based:

```text
Summary
Source
Evidence
Cause
Results
Raw
Permissions
```

The default view should be compact and readable. Raw trace should stay one
click deeper unless the object itself is raw output.

В Web baseline `0.2.9` session surface разделяет два уровня наблюдаемости:

- `Conversation` — основной рабочий режим. Системный bootstrap до первого
  пользовательского сообщения и работа между user message и terminal outcome
  представлены отдельными компактными disclosure groups. Активный turn раскрыт
  автоматически, дополняется provider activity в реальном времени и явно
  показывает отсутствие новых событий после bounded интервала;
- `Trace` — session-wide подробная летопись, доступная переключателем вместо
  чата и сохранённая в URL через `agentView=trace`. Здесь остаются coarse causal
  steps, raw event log и isolated Deduction;
- terminal events (`turn.completed`, interruption, block, runtime error) закрывают
  live group. Provider activity доставляется маленькими live frames и затем
  повторяется через durable bounded batches; Core idempotently принимает
  повторные event ids;
- Deduction, raw payload и raw event bodies по умолчанию находятся на один
  disclosure level глубже. Они не конкурируют с основным conversation flow за
  визуальный вес.

Такое разделение не отменяет source/cause graph: reference action остаётся на
message, grouped activity и trace steps, а Context Inspector показывает
человекочитаемый resolution status и раскрываемый raw payload.

В Web baseline `0.2.6` общий Context Inspector закрыт по умолчанию и вообще не
монтируется при пустом reference stack. Он резервирует отдельную колонку только
для выбранного reference на широком desktop, а на узком desktop открывается как
drawer. Sidebar state и Inspector stack независимы; последний pop или `Escape`
полностью освобождает место основной surface.

### Minimum V01 readiness model

V01 does not need perfect causality or the full workspace inspector. It needs
enough structure that later workspace, trace and artifact slices can attach
review evidence without reshaping the product model.

Required baseline:

- refs for session, runtime, turn, message, command, event, approval, warning
  and artifact placeholder;
- reserved ref shapes for future tool call, terminal command, output range, file
  range, diff hunk and check result;
- answer-level `open steps` link when source/cause refs exist;
- coarse agent step parsing from provider/runtime events where available;
- artifact and status links back to command/event/message where known;
- raw fallback for every unresolved rich view;
- `copy reference`;
- inspector/detail stack for linked refs;
- unknown/missing cause state instead of fake precision.

Explicitly not required for V01:

- command/output history beyond provider/session output needed for chat;
- file, terminal, diff and check surfaces;
- perfect line-level causality for every edit;
- full graph visualization;
- automatic reasoning over all logs;
- universal external source reconstruction;
- executable generated UI for explanations;
- proof-like provenance for every sentence in an answer.

### Deduction protocol and structured-explanation tool

`Deduction` is a review aid, not a hidden side channel or a claim that Uprava
has reconstructed the agent's reasoning. Core validates the provider result
against the `DeductionBlock` schema and emits `deduction.requested`,
`deduction.completed` or `deduction.invalid` events. The last case retains a
safe text/raw fallback.

Input rules:

- every material step should carry at least one source/evidence/cause/result
  ref;
- unreferenced claims must be marked as inference, assumption, alternative or
  unknown;
- raw logs should be summarized but remain linked;
- partial deductions are valid when their limitations are explicit;
- Core validates refs, permissions and the JSON shape;
- Core stores provenance: model/provider/session/schema version.

Output behavior:

- create a `DeductionBlock`; persist it as a `CausalityNarrative` only when
  the user or workflow needs durable review/handoff evidence;
- expose each step as an addressable `StepBlock`;
- link steps back to original events and results;
- render fallback as Markdown/text if rich step renderer is unavailable.

The optional `uprava.emit_causality_narrative` style tool follows the same
validation and provenance rules. This gives a path from today's coarse trace to
an agent-readable explanation UX without requiring the system to infer perfect
causality automatically.

### Storage implications

Event log remains source-of-truth for system events. Cause links and narratives
are additional indexes/views over it.

Storage model should avoid copying large outputs into every link:

```text
raw output blob/event
-> addressable range refs
-> links
-> step/narrative views
```

For external systems, Uprava should store enough snapshot metadata to explain
what the user saw at the time:

```text
external entity ref
external snapshot/version
fetch event
permissions/auth context
open external action
```

### Permissions and failure modes

Refs are permission-scoped. Opening a link may result in:

```text
resolved
redacted
not found
not available because node is offline
not available because plugin/provider is disabled
not available because permission is missing
stale snapshot available
raw fallback only
```

Important failure rules:

- never silently drop source/cause links because a rich renderer failed;
- never show a restricted raw log through an unrestricted summary block;
- mark agent-authored explanations as interpretations when system evidence is
  incomplete;
- preserve copyable refs even when current UI cannot render the target;
- show missing cause as missing, not as a confident invented chain.

### Relationship with neighboring mechanisms

`A-004 Modular UI and work surface` provides:

```text
refs, blocks, detail views, aspects, commands, navigation and popups
```

`A-005 Dynamic UI from agents` provides:

```text
agent-created blocks/artifacts that must expose source/evidence/cause refs
```

`A-006 Visual rendering and artifact semantics` provides:

```text
source-of-truth, fallback, visual object refs and artifact semantics
```

`A-007 Plugins, Tool Registry and MCP strategy` should provide:

```text
tool schemas, renderer registrations, parsers, integration snapshots and
permissioned actions that create reliable refs
```

`A-009 Human-agent dual interface` will use:

```text
machine-readable refs, selected objects, source/cause links and explanation
tools so agents can reason over UI state without screenshot parsing
```

### Tests/evals/checklist

For every review-facing object:

- Can the user open source?
- Can the user open cause when known?
- Is missing cause represented honestly?
- Is there readable fallback if rich renderer is unavailable?
- Do refs survive reload/reconnect?
- Are permission failures explicit and safe?
- Can the user copy a stable ref?
- Can an internal agent inspect the selected object and its refs?
- Does the view reduce review cost compared with raw trace?

For structured explanations:

- Does every material step link to evidence or state that evidence is missing?
- Are agent-authored interpretations distinguishable from system-derived
  events?
- Does the block distinguish observed facts, inferences, assumptions, unknowns
  and alternatives?
- Does invalid structured output fall back safely instead of appearing as a
  trusted deduction?
- Can a reviewer drill down from narrative step to raw event/log/file range?
- Does the narrative avoid duplicating huge logs?
- Does updating the narrative preserve version/provenance?

## Рабочая формула

Go to Source and Causality UX is the Uprava mechanism for navigating from a
visible result to the source, evidence and cause behind it.

It starts small:

```text
answer -> steps -> tool/command/log/diff/artifact
```

It grows into a typed source/cause graph:

```text
visual object -> source refs -> evidence refs -> cause refs -> raw events
```

It should be local, inspectable and proportional. The user should not have to
read the whole trace to answer one question, but every important claim, change
or artifact should have a path back to the best available evidence.
