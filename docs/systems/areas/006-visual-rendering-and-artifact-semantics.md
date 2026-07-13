# A-006 Visual Rendering and Artifact Semantics

Статус: `working-position`

Этот документ фиксирует рабочую позицию по ключевой механике `A-006 Visual
Rendering and Artifact Semantics`.

Главная позиция: `A-006` не является списком новых блоков и не означает, что
каждая визуализация в Uprava должна становиться отдельным `Block` из `A-004`.
Это сквозное направление, которое описывает, **где и как Uprava превращает
текст, файлы, events, tool outputs, external refs and artifacts в визуальные,
inspectable, referenceable and sometimes editable representations**.

Некоторые visual objects будут отдельными blocks или artifacts. Но многие
будут вложенными renderer enhancements внутри уже существующих blocks,
viewers, editors or previews:

- Mermaid diagram внутри Markdown-ответа агента;
- UML diagram внутри Markdown file preview;
- diagnostics and inline refs внутри code editor;
- command boundaries and error markers внутри terminal viewer;
- semantic grouping and impact markers внутри diff viewer;
- rich link preview внутри Markdown/text content;
- chart внутри generated dashboard artifact.

Поэтому `A-006` нужно понимать как слой **visual semantics**, а не как ownership
над каждым renderer implementation.

Документ намеренно не делит направление на версии поставки. Scope конкретных
итераций должен определяться отдельно. Здесь фиксируется общая модель, чтобы
Markdown rendering, editor previews, terminal/diff views, artifacts, dynamic UI
and external embeds не развивались как набор несовместимых локальных решений.

## Vision

### Какую проблему решает механика

Uprava должен быть work surface для agent workloads, а не chat transcript with
panels. Пользователь должен видеть работу в форме, которая лучше всего снижает
стоимость понимания, review and decision:

- source code лучше смотреть в editor with diagnostics, refs and previews;
- Markdown лучше смотреть с rendered diagrams, tables and links;
- terminal output лучше смотреть с command boundaries, status and errors;
- diff лучше смотреть с hunks, ownership, checks and impact markers;
- trace лучше смотреть как timeline или cause graph, а не как raw log;
- test results лучше смотреть как report with failed cases and links;
- dependency relationships лучше смотреть как graph;
- metrics лучше смотреть как chart/table/dashboard;
- external systems лучше смотреть как preview/snapshot/reference, а не только
  как URL.

Но эти visual representations имеют разные источники и разные lifecycle.
Mermaid внутри Markdown-файла не является тем же самым, что agent-generated
dashboard artifact. Test report из tool output не является тем же самым, что
inline preview в code editor. Grafana preview не является тем же самым, что
controlled external embed.

Если не задать общую модель, Uprava быстро получит много частных renderer-ов,
которые:

- не имеют stable references;
- не умеют fallback;
- не объясняют source-of-truth;
- не дают go to source/cause;
- по-разному работают с permissions;
- теряют состояние при reload/reconnect;
- непонятны internal agent-у;
- неясно становятся artifacts или остаются ephemeral views.

`A-006` решает эту проблему: оно описывает общую семантику visual objects
независимо от того, встроены они в Markdown renderer, code editor, terminal
viewer, tool-rendered block, artifact viewer или external preview.

### Главная модель

Каждый visual object в Uprava должен отвечать на один и тот же набор вопросов:

```text
source-of-truth:
  chat text | file | event stream | tool output | artifact snapshot |
  external ref | generated artifact/package

rendering scope:
  inline fragment | content enhancement | viewer/editor enhancement |
  block | artifact viewer | external preview/embed

addressability:
  can it be referenced, copied, selected, opened, linked, mentioned?

actions:
  view only | edit source | update artifact | invoke command |
  open detail view | export | open external

fallback:
  raw text | code fence | metadata | static snapshot | external link |
  render error with source range

ownership:
  Core | plugin | tool | external system | generated artifact

trace/cause:
  file range | message range | event | tool call | artifact version |
  external entity
```

Короткая формула:

```text
A-006 defines how renderable things behave as Uprava visual objects.
It does not require every visual object to be a separate block or artifact.
```

### Visual object vs block vs artifact

Нужно явно разделять три понятия.

#### Visual object

Visual object - любой визуально отрендеренный объект, который Uprava может
показать и, желательно, адресовать.

Примеры:

- Mermaid diagram внутри chat message;
- UML diagram внутри Markdown file preview;
- failed test row внутри test report;
- highlighted command segment внутри terminal viewer;
- dependency edge внутри graph;
- rich preview для GitHub issue URL;
- chart внутри dashboard.

Visual object может быть вложенным. Он не обязан быть самостоятельным block.

#### Block

Block - композиционная единица A-004 work surface. Chat message, markdown
viewer, code editor, terminal viewer, diff viewer, test report card or artifact
preview могут быть blocks.

Visual object может жить внутри block:

```text
ChatMessageBlock
  -> MarkdownRenderer
    -> MermaidInlineVisualObject
```

#### Artifact

Artifact - durable work/result object с identity, metadata, storage, refs,
permissions and lifecycle.

Не каждый visual object является artifact. Mermaid diagram, отрендеренный из
Markdown file range, может оставаться view. Но пользователь или agent может
создать artifact из visual object:

```text
Markdown source range
-> rendered diagram view
-> pin/export/save as DiagramArtifact
```

### Почему это отдельное направление

`A-006` нужно выделить отдельно, потому что visual rendering пересекает сразу
несколько механик:

- `A-004` дает surfaces, blocks, artifact viewers and renderer contribution
  points;
- `A-005` дает dynamic UI and generated UI artifacts from agents;
- `A-007` регистрирует plugin-provided renderers, artifact types and external
  integrations;
- `A-008` использует visual object refs для go to cause;
- `A-009` требует machine-readable UI state for agents.

Если оставить visual semantics внутри каждого из этих документов, появится
разрыв:

- Markdown renderer будет иметь одну модель refs;
- code editor - другую;
- test report - третью;
- external previews - четвертую;
- generated dashboards - пятую.

Для Uprava это опасно. Визуальный слой должен быть неоднородным по UI, но
однородным по базовым semantics: source, address, actions, fallback, cause,
permissions.

## Rendering Classes

Это не roadmap и не список реализаций. Это классы мест, где visual rendering
может появляться в Uprava.

### Class A: Inline render enhancement

Inline render enhancement - визуализация внутри уже существующего content
renderer-а. Она не становится отдельным A-004 block.

Примеры:

```text
chat message markdown -> Mermaid diagram inline
chat message markdown -> table rendering
chat message markdown -> file/reference chip
markdown file preview -> UML diagram inline
markdown file preview -> rich link preview
issue/comment text -> external entity preview
```

Source-of-truth обычно:

```text
chat message text
file content + source range
external URL/entity ref embedded in text
```

Ключевая семантика:

- visual object должен знать source range or message range;
- fallback - raw Markdown/code fence/link;
- render errors должны указывать на source fragment;
- actions чаще всего: copy, open source, open detail, export, pin as artifact;
- edits должны редактировать source text, а не hidden renderer state.

Пример Mermaid в agent chat:

```text
SessionMessageBlock
-> MarkdownRenderer
-> fenced mermaid block detected
-> MermaidRenderer renders inline diagram
-> source_ref = session_message_id + text_range
-> fallback = original code fence
```

Это не dynamic UI из A-005 само по себе. Даже если текст написал агент,
визуализация здесь является Markdown render enhancement. A-005 начинается там,
где агент создает explicit dynamic block/artifact proposal, а не просто пишет
renderable Markdown.

### Class B: Viewer/editor enhancement

Viewer/editor enhancement - визуализация внутри специализированного viewer or
editor surface.

Примеры:

```text
code editor -> diagnostics, references, blame, inline previews
markdown editor -> diagram preview beside source
diff viewer -> semantic hunk grouping, risk/impact markers
terminal viewer -> command boundaries, exit statuses, error highlighting
trace viewer -> event grouping and compact summaries
file browser -> status badges and generated previews
```

Здесь визуализация является частью поведения parent viewer/editor. Ее не нужно
насильно выносить в отдельный block.

Source-of-truth:

```text
file content
working tree state
language/tool diagnostics
terminal events
diff metadata
trace/event log
```

Ключевая семантика:

- visual object может быть addressable через file range, diff hunk, terminal
  segment or event id;
- actions должны идти через parent viewer/editor commands;
- edits должны менять source file, workspace state or artifact state;
- fallback - базовый editor/viewer без enhancement;
- renderer errors не должны ломать основной viewer/editor.

Пример:

```text
CodeEditorBlock
-> file range diagnostic marker
-> action: open test failure
-> cause_ref: check event + compiler/test output
```

### Class C: Dedicated visual block or artifact

Dedicated visual block/artifact - самостоятельная visual единица внутри A-004
surface.

Примеры:

```text
test/check report
trace timeline
terminal replay artifact
dependency graph
causality map
generated diagram artifact
dashboard artifact
image/table/chart artifact
```

Здесь visual object часто является block или artifact, потому что у него есть
самостоятельная identity, actions and review value.

Source-of-truth:

```text
tool output
event stream
artifact snapshot
workspace analysis
external data snapshot
generated artifact data model
```

Ключевая семантика:

- artifact identity and versioning where needed;
- source refs and cause refs mandatory for review-facing objects;
- fallback snapshot or structured metadata;
- explicit permissions;
- export/share actions where useful;
- can open in artifact viewer/detail view.

### Class D: Tool-rendered visual result

Tool-rendered visual result связан с A-005 and A-007.

Примеры:

```text
run_tests -> test report view
query_metrics -> chart/table
analyze_dependencies -> dependency graph
search_issues -> issue list preview
profile_command -> performance timeline
```

Creation path belongs to A-005/A-007:

```text
registered tool
-> tool call
-> result
-> renderer contract
-> visual block/view
```

Но visual semantics belongs to A-006:

- how test reports behave;
- how charts fallback;
- how rows link to source/cause;
- how result snapshots are stored;
- how action refs are represented;
- how tool output becomes artifact or remains view.

### Class E: Generated visual artifact

Generated visual artifact связан с A-005.

Примеры:

```text
agent-generated dashboard
agent-generated form/wizard
agent-generated calculator
agent-generated architecture diagram
agent-generated simulation
```

A-005 отвечает за то, как agent может создать dynamic UI or generated
artifact. A-006 отвечает за visual semantics:

- chart/table/dashboard behavior;
- form visual/validation behavior;
- diagram source/display relationship;
- fallback representation;
- addressability;
- artifact promotion;
- snapshot/export semantics.

### Class F: External visual reference or embed

External visual reference/embed - visual representation of external system.

Примеры:

```text
Grafana dashboard
GitHub issue/PR/check
Linear issue/project
CI run
log explorer
monitoring panel
cloud resource view
```

Preferred ladder:

```text
external link
-> rich preview
-> artifact snapshot
-> controlled embed
```

External embed should be rare and explicitly permissioned. Often Uprava-native
preview or snapshot gives better traceability than iframe.

## Architecture

### Rendering layers

Uprava visual rendering should be described in layers:

```text
Surface renderer
  renders a known workbench area or route

Block renderer
  renders a compositional A-004 block

Content renderer
  renders content inside a block, e.g. Markdown

Inline fragment renderer
  renders embedded source fragments, e.g. Mermaid fence

Viewer/editor enhancement renderer
  adds visual semantics inside code/diff/terminal/file viewers

Artifact viewer
  renders durable artifact identity/state/version

External preview/embed renderer
  renders external refs with permission and fallback
```

The same visual type can appear at different layers.

Example:

```text
Mermaid diagram:
  inline fragment inside chat Markdown
  inline fragment inside Markdown file preview
  detail view from source range
  standalone diagram artifact after pin/export
```

### Visual object descriptor

Not every visual object needs to be persisted as a database row. But every
meaningful visual object should be describable by a common shape.

```text
VisualObjectDescriptor:
  visual_object_id optional
  visual_kind
  parent_ref
  source_ref
  render_scope
  renderer_id
  renderer_kind
  title optional
  state
  actions
  permissions
  fallback
  cause_refs
  artifact_ref optional
```

`visual_object_id` can be stable or derived:

```text
stable artifact visual -> stored id
file range visual      -> derived from file ref + range + renderer
message range visual   -> derived from message id + range + renderer
event visual           -> derived from event id + renderer
```

### Source refs

Source refs are the backbone of A-006.

```text
SourceRef:
  kind:
    chat_message_range
    file_range
    file_snapshot_range
    diff_hunk
    terminal_segment
    event_ref
    tool_call_ref
    artifact_version_ref
    external_entity_ref
    generated_package_ref
  id
  range optional
  version optional
```

Rules:

- source-backed visuals should support go to source;
- generated artifacts should support go to originating event/tool/agent work;
- external previews should support open external and snapshot provenance;
- render errors should attach to source refs;
- if source changed, visual object should be marked stale or re-rendered from
  the current source.

### Render scope

Render scope describes where a visual object lives.

```text
RenderScope:
  inline_fragment
  content_enhancement
  viewer_enhancement
  block
  artifact_viewer
  detail_view
  external_preview
  external_embed
```

This avoids forcing every visualization into the A-004 block model. A
`MermaidInlineDiagram` and a `DependencyGraphArtifact` may share renderer
concepts, but their scope and lifecycle differ.

### Artifact promotion

Visual objects can be promoted to artifacts when they need durable identity.

Promotion examples:

```text
rendered Mermaid file range -> DiagramArtifact
terminal command segment    -> TerminalReplayArtifact
test output                 -> TestReportArtifact
chart from query result     -> ChartArtifact
external dashboard preview  -> SnapshotArtifact
```

Promotion should record:

```text
source_ref
created_from_visual_kind
artifact_type
snapshot/rendered_payload
source_version
trace_refs
permissions
fallback
```

Promotion does not mean copying all source forever by default. Sometimes
artifact stores a snapshot; sometimes it stores a source ref plus render
metadata; sometimes it stores both.

### Renderer ownership

Renderers can be owned by different parts of Uprava:

```text
Core renderer
  markdown, code/diff/terminal basics, core artifacts

Plugin renderer
  Mermaid, PlantUML, Grafana preview, GitHub/Linear preview

Tool renderer
  visual representation of registered tool output

Dynamic UI renderer
  schema-driven/generated artifact renderer from A-005

External renderer/embed
  controlled iframe or external preview runtime
```

A-006 does not require a renderer to live in one subsystem. It requires that
renderer-visible outputs conform to Uprava visual semantics:

- source refs;
- fallback;
- actions;
- permissions;
- render errors;
- addressability where useful;
- agent-readable metadata where useful.

### Renderer contract

Renderer contracts are shared with A-004/A-005/A-007, but A-006 defines the
visual behavior requirements.

```text
VisualRendererContract:
  renderer_id
  visual_kinds
  accepted_source_kinds
  render_scopes
  input_schema
  output_visual_descriptor_schema
  actions
  fallback_strategy
  error_model
  trust_level
  permissions
```

For inline renderers, the contract can be lightweight:

```text
MermaidRenderer:
  accepted_source_kinds: [markdown_code_fence]
  render_scopes: [inline_fragment, detail_view, artifact_viewer]
  fallback_strategy: show_code_fence_with_error
  actions: [copy_source, open_detail, export_svg, pin_as_artifact]
```

For artifact renderers, the contract is stronger:

```text
TestReportRenderer:
  accepted_source_kinds: [tool_call_ref, artifact_version_ref]
  render_scopes: [block, artifact_viewer, detail_view]
  fallback_strategy: summary_table_and_raw_output
  actions: [open_failed_test, rerun_failed, copy_command, go_to_cause]
```

### Fallback and errors

Fallback is mandatory for visual rendering.

Fallback examples:

```text
Mermaid render error -> show source code fence + line/error
PlantUML unavailable -> show source + install/enable renderer if allowed
chart data invalid   -> show table/metadata + validation error
terminal replay fail -> show raw output with segment markers
external preview fail -> show URL and last snapshot metadata
plugin disabled      -> show unknown visual object fallback
```

Renderer failure should not break parent content. A Markdown message with one
bad diagram should still render the rest of the message.

### Actions

Visual object actions should be explicit and scope-aware.

Common actions:

```text
open_source
open_detail
copy_source
copy_reference
export
pin_as_artifact
edit_source
update_artifact_state
invoke_command
open_external
go_to_cause
```

Action rules:

- editing source-backed visual object edits the source, not hidden visual
  state;
- artifact-backed visual object can update artifact state/version;
- external visual object opens external system or stored snapshot;
- privileged actions go through Core command/permission layer;
- action availability can depend on render scope and trust level.

### Agent-readable visual state

A-006 must support the A-009 goal that UI is readable by agents.

Agent-readable representation should include:

```text
visible visual objects
visual kinds
source refs
artifact refs
selected object
render errors
available actions
permissions
summary metadata
```

The agent should not need screenshot interpretation to know that a Markdown
message contains a Mermaid diagram with a render error, or that a test report
has three failed tests linked to file ranges.

### Relationship with A-004 Modular UI and Work Surface

`A-004` owns where renderers are mounted:

```text
surfaces
blocks
artifact viewers
detail views
contribution points
commands
navigation model
```

`A-006` owns how visual objects behave once rendered:

```text
source-of-truth
render scope
addressability
fallback
actions
artifact promotion
visual semantics
```

Not every A-006 visual object is an A-004 block. Many are nested inside block
renderers.

### Relationship with A-005 Dynamic UI from Agents

`A-005` owns agent-created dynamic UI lifecycle:

```text
agent proposal
tool-rendered block
declarative generated UI
generated UI artifact
sandboxed app artifact
```

`A-006` owns visual semantics used by those objects:

```text
chart behavior
table behavior
diagram behavior
dashboard source/fallback/action semantics
form visual/validation semantics
artifact promotion and rendering scope
```

If an agent writes Mermaid in Markdown, that is usually A-006 inline rendering,
not A-005 dynamic UI. If an agent explicitly proposes a generated dashboard
artifact, creation belongs to A-005 and visual semantics belong to
A-006.

### Relationship with A-007 Plugins, Tool Registry and MCP Strategy

Plugins and tools provide many renderers, but A-006 defines the visual contract
they should satisfy.

Plugin Registry should be able to describe:

```text
provided visual renderers
accepted source kinds
render scopes
artifact types
fallback behavior
external origins
permissions
trust level
```

Tool Registry should be able to connect:

```text
tool output schema
-> visual renderer
-> visual object descriptor
-> artifact type optional
-> actions/fallback/cause refs
```

### Relationship with A-008 Go to Source and Causality UX

A-006 provides the visible entry points for go to source/cause.

Examples:

```text
Mermaid diagram -> source file/message range
failed test row -> test command event + source file range
diff marker -> related check/tool/agent decision
chart point -> query result row + data source
terminal segment -> command event + runtime session
external preview -> external entity + snapshot event
```

Go to Source / Cause should work from inline visuals, viewer enhancements,
blocks and artifacts. It should not be limited to top-level A-004 blocks.

### Relationship with A-009 Human-Agent Dual Interface

A-006 helps make visual UI available to agents without requiring screenshot
parsing.

For example, internal Uprava agent should be able to answer:

- "Какие диаграммы есть в этом Markdown?"
- "Где source для этой UML diagram?"
- "Почему этот chart не отрендерился?"
- "Какие failed tests видны в report?"
- "Какие visual objects были generated by agent, а какие source-backed?"

### Quality questions

For every visual object type:

- What is the source-of-truth?
- Is it a visual object, block, artifact, or nested enhancement?
- Can the user go to source or cause?
- What happens if renderer/plugin is unavailable?
- Is fallback readable and useful?
- Are actions explicit and permissioned?
- Does editing change source, artifact state, or external system?
- Is the visual object readable by internal agents?
- Can it survive reload/reconnect if it carries review value?
- Does it reduce review/understanding cost compared with raw text?

## Рабочая формула

Visual Rendering and Artifact Semantics is the Uprava mechanism for making
files, messages, events, tool outputs, external refs and artifacts visually
understandable without forcing every visualization to become a separate block.

It defines visual object semantics:

```text
source-of-truth
rendering scope
addressability
actions
fallback
ownership
trace/cause refs
artifact promotion
agent-readable state
```

A-006 does not own every renderer implementation. It defines the rules that
renderers should follow so Markdown diagrams, code editor previews, terminal
segments, diff markers, test reports, charts, dashboards, generated artifacts
and external previews behave like coherent Uprava visual objects.
