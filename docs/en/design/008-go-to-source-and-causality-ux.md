# A-008 Go to Source and Causality UX

Статус: `draft`

Этот документ фиксирует корневую позицию по ключевой механике `A-008 Go to
Source and Causality UX`.

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

### Главная модель

Любой важный видимый объект Cortex должен иметь stable reference and optional
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
- или Cortex построит огромный trace graph, который теоретически точный, но
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

Cortex показывает detail block:

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

Cortex раскрывает:

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

#### 5. Агент объясняет, что произошло, через Cortex tool

Отдельная механика: Cortex может дать агенту skill и tool, которые позволяют
сформировать structured explanation of work.

Пользователь просит:

```text
Объясни по шагам, что произошло и что к чему привело.
```

Агент использует Cortex-facing skill:

```text
Read available events/refs.
Do not invent unreferenced steps.
Group low-level events into review-facing steps.
Mark assumptions and missing evidence.
Emit structured steps through Cortex tool.
```

И вызывает tool вроде:

```text
cortex.emit_causality_narrative
```

Tool принимает структурированный объект:

```text
CausalityNarrative:
  title
  scope_ref
  steps:
    - title
      summary
      source_refs
      evidence_refs
      cause_refs
      result_refs
      confidence
      notes
```

UI рендерит это как block/artifact with linked steps. Важно: такой narrative
не становится новым source-of-truth. Это agent-authored interpretation поверх
событий Cortex. Его ценность в том, что он группирует шумные events в
понятную цепочку, но каждый существенный шаг должен ссылаться на source,
evidence or explicitly say that evidence is missing.

### Agent-facing сценарии

Для internal Cortex agent and provider adapters `A-008` дает структурированный
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

#### CortexRef

Stable address на сущность, диапазон или визуальный объект.

```text
CortexRef:
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

#### CausalityNarrative

Structured explanation artifact или transient block, который группирует steps.

```text
CausalityNarrative:
  ref
  scope_ref
  title
  created_by
  created_from_refs
  steps
  unresolved_questions
  limitations
```

Если narrative влияет на review/handoff, его стоит сохранять как artifact
version. Если это quick popup explanation, он может быть transient view over
events.

### Границы ответственности

Core owns:

- stable refs;
- event log and trace metadata;
- system-derived links;
- permissions for resolving refs;
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
- mapping provider-specific ids to Cortex refs when possible.

Web owns:

- rendering links, popovers, detail views and drilldown stack;
- source/cause action presentation;
- safe fallback for unresolved refs;
- navigation state and selected object;
- visual distinction between source, evidence, cause and related links.

Agent/tool owns:

- optional structured causality narratives;
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

### Structured explanation tool

The `cortex.emit_causality_narrative` style tool should be designed as a
review aid, not as a hidden side channel.

Input rules:

- every material step should carry at least one source/evidence/cause/result
  ref;
- unreferenced claims must be marked as assumptions or interpretations;
- raw logs should be summarized but remain linked;
- tool should accept partial narratives;
- tool should validate refs and permissions;
- tool should store provenance: model/provider/session/tool version.

Output behavior:

- create or update a `CausalityNarrative` block/artifact;
- emit event for the narrative;
- expose each step as addressable `StepBlock`;
- link steps back to original events and results;
- render fallback as Markdown/text if rich step renderer is unavailable.

This gives a path from today's coarse trace to tomorrow's agent-readable,
structured explanation UX without requiring the system to infer perfect
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

For external systems, Cortex should store enough snapshot metadata to explain
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
- Can a reviewer drill down from narrative step to raw event/log/file range?
- Does the narrative avoid duplicating huge logs?
- Does updating the narrative preserve version/provenance?

## Рабочая формула

Go to Source and Causality UX is the Cortex mechanism for navigating from a
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
