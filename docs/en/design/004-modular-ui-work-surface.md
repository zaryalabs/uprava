# A-004 Modular UI and Work Surface

Статус: `working-position`

Этот документ фиксирует рабочую позицию по ключевой механике `A-004 Modular UI
and work surface`.

## Главная позиция

На текущий момент `A-004` выглядит не как `Notion-like UI`, а как
**модульная рабочая поверхность Cortex**: стабильный workbench-shell, внутри
которого можно расширять блоки, detail views, панели, ссылки, previews,
artifact views and actions.

Cortex не должен давать плагинам произвольно перестраивать весь интерфейс.
Более подходящая модель:

```text
IDE/workbench shell
+ Obsidian-like links/plugins/navigation
+ Notion-like typed blocks
+ VS Code-like contribution model
+ Atom-like package/service model
+ Vim-like addressable object/action model
+ Cortex-native artifacts/trace/causality
```

Интерфейс остается предсказуемым, но внутренние поверхности становятся
расширяемыми.

`A-010 Project Workspace Surface` является конкретной post-V01 workbench
поверхностью внутри этой модели: file tree, file viewer/editor, terminal/PTY,
command history, diff/check entry points and workspace refs. `A-004` отвечает за общий
workbench-shell, commands/contributions and extension points; `A-010` отвечает
за workspace-specific lifecycle, file editing, terminal access, Node/Core
boundary, permissions and traceability.

## Базовые сущности

Пока складывается такая модель:

```text
Surface = известное место в UI
Block = визуальный модуль внутри surface
Artifact = долговечный результат работы
Reference = адрес на сущность/элемент/диапазон
Detail View = экран/попап/inspector по reference
Aspect = подвкладка/срез внутри detail view
Command = именованное действие, которое можно вызвать из UI, palette, hotkey or agent
Contribution = декларативное подключение plugin-функции в известное место UI
Context = текущий addressable object, selection, permissions and runtime state
Service = capability, которую один plugin/core module предоставляет другим
Plugin = пакет, который добавляет tools, renderers, links, aspects, commands, services, artifact types
Navigable Object = объект UI, по которому можно двигаться keyboard/navigation commands
```

Важная граница:

```text
Tool вызывает действие.
Artifact хранит результат.
Block показывает результат.
Reference связывает результат с причиной/контекстом.
Detail View раскрывает объект.
Command выполняет действие над текущим context.
Contribution подключает plugin к известному extension point.
Service дает переиспользуемую capability другим частям системы.
Plugin добавляет новые типы всего этого.
Navigable Object делает UI управляемым как структурированный документ.
```

## Модульность UI

Модульность должна быть не только в том, что можно добавить `MermaidBlock` или
`ChartBlock`.

Она должна быть глубже:

- можно добавить новый renderer блока;
- можно добавить новый тип artifact;
- можно добавить link handler;
- можно добавить detail view для сущности;
- можно добавить aspect внутри существующего detail view;
- можно добавить action в известный surface;
- можно добавить preview для внешней системы;
- можно добавить command, доступную из palette, hotkey, block action или agent action;
- можно добавить context-aware contribution, которая появляется только при нужных условиях;
- можно добавить service, который используют другие blocks/plugins/core modules.

Это дает путь к Mermaid, special links, GitHub/Linear previews, test reports,
UML, dashboards и будущему dynamic UI.

## Lifecycle from output to UI

Чтобы модульность не осталась только frontend-паттерном, нужен понятный
lifecycle от plugin/tool/agent output до видимого UI.

Базовый flow:

```text
Core/plugin module registers contribution
-> Tool/agent/runtime emits output, artifact or block descriptor
-> Core validates type, refs, permissions and trace metadata
-> Core stores artifact/block/event metadata
-> Web resolves renderer, detail view, actions and navigation model
-> User invokes action through command registry
-> Core routes command to Node/external provider/plugin service
-> Result returns as event/artifact/block update
```

Это важно: Web не должен сам решать, можно ли выполнить action, увидеть
artifact или открыть privileged detail view. Web показывает то, что разрешено
Core-level registry, permissions and context.

Минимальный block descriptor может мыслиться так:

```text
BlockDescriptor:
  block_id
  type
  surface_id
  artifact_ref optional
  entity_ref optional
  renderer_id
  props
  actions
  trace_refs
  required_permissions
  navigation_model optional
```

Если renderer отсутствует, disabled или не прошел permission check, UI должен
показать safe fallback:

```text
unknown block type
artifact metadata
raw text or sanitized payload if safe
open external/source action if allowed
copy reference
```

Так Cortex сохраняет traceability and reviewability даже тогда, когда rich UI
недоступен.

## Уроки VS Code, Atom and Vim

VS Code, Atom and Vim важны не как редакторы кода, а как разные модели
модульности. Для Cortex полезнее не копировать их UI, а взять их
архитектурные паттерны.

### Что берем из VS Code

VS Code дает сильную модель **manifest-driven contributions**:

- extension points через manifest;
- command registry, где почти любое действие является командой;
- context keys and `when` clauses;
- views, panels, custom editors and webviews как контролируемые surface types;
- extension host отдельно от workbench UI;
- language-server-style разделение: domain logic живет отдельно, UI получает
  структурированные capabilities/results.

Для Cortex это ложится так:

```text
Plugin manifest:
- tools
- commands
- block renderers
- artifact types
- detail views
- aspects
- link handlers
- menu/action contributions
- activation conditions
- permissions
```

Ключевая идея: plugin не меняет интерфейс напрямую. Он contributes into known
places.

Пример:

```text
contributes.action:
  surface: artifact.viewer.toolbar
  when: artifact.type == "test-report"
  command: testReport.openFailedCase
```

Это хорошо подходит Cortex, потому что сохраняет предсказуемость workbench и
при этом позволяет добавлять новые возможности.

### Что берем из Atom

Atom был более hackable package system. Оттуда полезны:

- package как единица расширения;
- services: один package предоставляет capability, другой ее потребляет;
- pane/item model: UI состоит из открываемых items во вкладках/panes;
- высокая локальная расширяемость для power users and internal teams.

Анти-урок Atom: если package может менять DOM/CSS/behavior почти как угодно,
система становится хрупкой, плохо управляемой и тяжелой для поддержки.

Для Cortex стоит взять package/service model, но не брать произвольный
monkey-patching.

Пример:

```text
Plugin provides:
- renderer service
- parser service
- preview service
- external entity resolver
- artifact importer
```

Mermaid plugin может предоставить parser, renderer and artifact type. GitHub
plugin может предоставить entity resolver, previews and actions.

### Что берем из Vim

Vim важен не визуальной системой, а композицией primitives:

- все адресуемо: buffer, window, range, mark, register;
- действия работают над текущим context/selection;
- commands, mappings and hooks;
- composability: action + object;
- маленький стабильный core и огромная plugin ecosystem вокруг него.

Для Cortex это особенно полезно для Go to Cause and context actions:

```text
current entity + current selection + available commands
```

Например пользователь находится на `diff hunk`. Тогда доступны:

```text
open cause
open related command
ask agent about this hunk
create follow-up task
accept hunk
reject hunk
copy reference
```

То есть command palette and context actions должны работать не с абстрактным
экраном, а с текущим addressable object.

### Combined editor-inspired model

Итоговая формула:

```text
VS Code contribution model
+ Atom package/service model
+ Vim addressable/composable object-action model
```

Для Cortex это означает:

- стабильный workbench-shell;
- manifest-driven extension points;
- command registry;
- context-aware actions;
- typed surfaces;
- plugin-provided services;
- addressable entities/ranges/blocks;
- semantic keyboard navigation;
- drilldown stack;
- keyboard/command-palette-first interaction;
- controlled escape hatch для webview/iframe/custom renderer.

Чего лучше не брать:

- Atom-style полную изменяемость DOM/CSS;
- arbitrary plugin JS в основном React tree;
- Vim-style скрытую глобальную магию, где состояние трудно понять;
- VS Code-style перегруженность extension API слишком рано.

Для V01 и ближайших queue-срезов лучше начать с **internal extension architecture**, даже если
external plugins еще нет. Сам Cortex должен строиться так, как будто он уже
расширяемый:

```text
core.chatMessageBlock
core.terminalCommandBlock
core.diffBlock
core.traceEventDetail
core.fileRangeReference
core.markdownRenderer
core.mermaidRenderer
```

Позже external Plugin Registry начнет регистрировать такие же contributions.

## Addressable UI

Почти все важное в Cortex должно быть адресуемым:

```text
session
turn
message block
artifact
diff hunk
file range
terminal command
tool call
trace event
approval request
warning
external entity
```

Reference не должен быть ссылкой на случайный DOM-node. Это стабильный,
permission-scoped адрес на продуктовую сущность, artifact, event, range or
block.

Хороший reference должен быть:

- serializable;
- copyable/shareable where allowed;
- resolvable through Core;
- usable in Web, mobile and future agent-readable UI;
- able to degrade to metadata/fallback if rich renderer is unavailable;
- connected to trace/cause links when relevant.

Ссылка на такой объект открывает не обязательно новую страницу, а
контролируемый **drilldown stack**: popover, drawer, inspector или mobile
screen.

На desktop это может быть правый inspector, stacked panels или peek windows.
На mobile это естественно превращается в stack экранов: открыл причину,
провалился глубже, вернулся назад.

Важно не превращать это в хаос произвольных floating windows. Лучше иметь
контролируемый стек:

```text
Main work surface
  Inspector stack:
    Diff hunk
      Caused by tool call
        Caused by terminal command
          Logs
          Files touched
          Agent message
```

## Semantic keyboard navigation

Еще одна важная Vim-inspired идея: Cortex UI должен быть не только
addressable, но и **keyboard-navigable as a structured document**.

В вебе это возможно, но не через обычный browser tab order. Нужен собственный
semantic navigation layer, где весь UI представляет себя как набор navigable
objects:

```text
panel
tree item
block
artifact
diff hunk
file line/range
trace event
git status item
button/action
link
```

Поверх этого работает navigation engine:

```text
current object
+ current surface/panel
+ current selection
+ movement command
-> next object
```

То есть `j/k`, стрелки или другие movement commands двигают не DOM focus туда,
куда браузер решил, а Cortex-level cursor по смысловым объектам.

Пример поведения:

```text
left: file tree
center: session timeline / markdown / artifacts
right: inspector
bottom: git/checks/terminal
```

Пользователь может:

- `j/k` двигаться по элементам текущей панели;
- `h/l` уходить на parent/child или соседнюю панель;
- `Tab` / `Shift+Tab` переключать крупные surfaces;
- `Enter` открыть selected object;
- `o` открыть detail view;
- `g c` сделать go to cause;
- `:` открыть command palette;
- `/` искать внутри текущей surface;
- `Esc` вернуться из input/edit mode в navigation mode.

Это не обязательно должно быть Vim-копией. Важен принцип: UI становится
navigable document, где panels, trees, blocks and artifacts ведут себя как
структурированный текст.

Технически каждый surface or block должен уметь объявить свою navigation
model:

```text
NavigableObject:
  ref: CortexRef
  surface_id
  parent_ref optional
  children optional
  role: block | tree-item | diff-hunk | action | link | line
  order_key
  commands
```

Контракт:

```text
Surface registers navigation model.
Block registers its internal navigable objects.
Plugin block exposes navigation metadata if it wants first-class keyboard UX.
Global navigation engine resolves movement.
Command registry resolves actions for current object.
```

Для accessibility это нужно совмещать с нормальным focus management:
`roving tabindex`, `aria-activedescendant`, visible focus ring and predictable
screen reader labels. Нельзя ограничиться нарисованной подсветкой `div`, иначе
UI станет недоступным.

Главная сложность: input fields, editors, terminals and embedded iframes.
Поэтому нужны modes:

```text
navigation mode = клавиши управляют Cortex UI
input mode = клавиши идут в textarea/editor/terminal
embedded mode = клавиши отданы iframe/webview, Esc возвращает Cortex focus
```

Для plugin blocks это становится частью extension contract. Если plugin хочет
быть first-class частью work surface, он должен описать свои navigable objects
and commands. Если это внешний embed, Cortex может навигироваться только до
границы embed-а.

## Go to Source / Cause

Go to Source / Cause тогда становится не отдельной feature, а частным случаем
общей reference/detail/aspect модели.

Пример:

```text
diff line
-> why?
-> tool call
-> command output
-> logs
-> file reads
-> agent decision
-> original user request
```

Так Cortex получает прозрачность без превращения UI в один огромный trace log.
Пользователь двигается от видимого результата к причинам, evidence and context
локально, по мере необходимости.

## Аспекты

Detail view лучше мыслить как контейнер с аспектами:

```text
Tool call detail:
- Summary
- Inputs
- Output
- Logs
- Related artifacts
- Permissions
- Cause links
- Raw event
```

Плагины смогут добавлять свои аспекты. Например GitHub plugin может добавить
для `pull_request` аспекты `Checks`, `Comments`, `Files`, `Review status`.

## Markdown и HTML

Markdown стоит принять как агентский формат ввода/вывода, но не как финальную
модель UI.

Агент может писать Markdown, Mermaid, tables, links. Cortex должен парсить это
в typed blocks and references.

HTML лучше ограничить:

- sanitized HTML block;
- no arbitrary scripts;
- rich interactivity only through registered block/render contracts;
- later sandboxed plugin/dynamic UI runtime.

## Responsibility and trust boundaries

Для A-004 важно сразу не смешать UI extensibility с execution permissions.
Рабочая граница такая:

```text
Core owns:
- Plugin Registry and Tool Registry;
- contribution manifests;
- permissions and context keys;
- artifact/block/event metadata;
- command authorization and routing;
- traceability of actions.

Web owns:
- rendering registered blocks/views/aspects;
- local layout state;
- keyboard navigation and focus management;
- command palette and context action presentation;
- safe fallbacks for unavailable renderers.

Node/Provider owns:
- local files, terminal, processes and credentials;
- tool execution near workspace or external system;
- raw logs or large local outputs when needed.
```

Правило: renderer показывает state and proposes actions, но privileged action
проходит через command registry and Core authorization. Даже если action
начинается в plugin block, она не должна обходить permissions, routing and
trace.

Для trust levels можно держать простую лестницу:

```text
core renderer
trusted bundled plugin renderer
installed local/team plugin renderer
sandboxed custom renderer
external embed
sanitized/raw fallback
```

Чем ниже доверие, тем меньше возможностей:

- меньше direct interactivity;
- меньше доступа к context;
- меньше прав на commands;
- сильнее sandboxing;
- более явный user consent;
- обязательный fallback.

Эта граница особенно важна для будущего `A-005 Dynamic UI from agents`: agent
может предложить UI, но не должен автоматически получить возможность исполнять
произвольный code in main React tree.

## Внешние системы

Полный embed вроде Grafana внутри Cortex лучше не делать базовым способом.
Он быстро тянет auth, iframe security, layout, permissions, чужую навигацию и
непредсказуемый UX.

Более безопасная queued model:

```text
external link
-> rich preview
-> artifact snapshot
-> controlled sandboxed embed only when justified
```

Cortex-native preview часто ценнее, чем iframe чужой системы.

Пример:

```text
Grafana dashboard link
  -> preview block
  -> incident/status artifact
  -> cause refs
  -> external open action
```

Полный embed нужен только если он реально уменьшает переключение контекста, а
не просто создает браузер внутри браузера.

## First release vs later

Эта модель широкая, но ее можно вводить постепенно.

### V01 baseline

V01 должен заложить internal extension architecture без полноценного
external plugin ecosystem:

- стабильный workbench-shell;
- небольшой набор core surfaces;
- встроенные core renderers for chat, markdown, status, approvals and safe
  fallbacks, with renderer contracts reserved for files, terminal, diff and
  trace;
- минимальный visual block/artifact contract;
- stable references for session, runtime, turn, message, command, event,
  approval, warning and artifact placeholder, plus reserved reference shapes for
  future file ranges, terminal output, diff hunks, checks and tool calls;
- первый inspector/detail stack;
- command registry for core actions;
- basic context actions;
- safe fallback for unknown block/artifact types;
- keyboard navigation хотя бы по core surfaces;
- no arbitrary external plugin JS in main React tree.

Главная цель V01: сам Cortex должен быть написан так, будто он уже
расширяемый, даже если пользовательские plugins еще не подключаются.

### Feature queue baseline

Feature queue может сделать modularity visible:

- Plugin Registry v1;
- manifest-driven contributions;
- plugin-provided tools, commands, link handlers and artifact types;
- basic plugin configuration UI;
- permission checks for plugin actions;
- first external previews, например GitHub/Linear;
- first non-core visual blocks;
- richer command palette and context keys.

### Later

Позже можно добавлять:

- sandboxed custom renderers;
- dynamic UI from agents;
- richer artifact gallery and layout;
- controlled external embeds;
- plugin-provided services used by other plugins;
- full semantic navigation across plugin blocks;
- mobile-first drilldown stack;
- team/cloud governance for plugin trust and permissions.

## Связь с соседними механиками

`A-004 Modular UI and work surface` отвечает:

```text
Где и как UI принимает модульные блоки, ссылки, panels, detail views, keyboard navigation and extension points?
```

`A-005 Dynamic UI from agents` отвечает:

```text
Кто и при каких условиях может создать новый UI на лету?
```

`A-008 Go to Source and Causality UX` отвечает:

```text
Как reference/detail/aspect модель используется для навигации к source,
evidence and причинам?
```

`A-007 Plugins, Tool Registry and MCP strategy` отвечает:

```text
Кто регистрирует tools, renderers, permissions, artifact types and UI contracts?
```

## Рабочая формула

Cortex work surface is a stable, addressable, keyboard-navigable, extensible
workbench.

Она не является произвольным page builder. Она состоит из typed surfaces,
blocks, artifacts, references, detail views, aspects, commands, contributions,
services and navigable objects.

Plugins расширяют известные extension points через manifest-driven
contributions. Commands разрешаются из текущего context: addressable object,
selection, permissions and runtime state. Navigation разрешается из текущего
surface, object, selection and mode.

Agents могут производить Markdown/structured output, который Cortex превращает
в blocks, artifacts and references. Go to Source / Cause использует ту же
модель, чтобы превращать любой видимый результат в вход к источнику, evidence
and цепочке причин.
