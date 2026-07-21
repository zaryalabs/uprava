# A-004 Modular UI and Work Surface

Статус: `working-position`

Этот документ фиксирует рабочую позицию по ключевой механике `A-004 Modular UI
and work surface`.

## Главная позиция

На текущий момент `A-004` выглядит не как `Notion-like UI`, а как
**модульная рабочая поверхность Uprava**: стабильный workbench-shell, внутри
которого можно расширять блоки, detail views, панели, ссылки, previews,
artifact views and actions.

Uprava не должен давать плагинам произвольно перестраивать весь интерфейс.
Более подходящая модель:

```text
IDE/workbench shell
+ Obsidian-like links/plugins/navigation
+ Notion-like typed blocks
+ VS Code-like contribution model
+ Atom-like package/service model
+ Vim-like addressable object/action model
+ Uprava-native artifacts/trace/causality
```

Интерфейс остается предсказуемым, но внутренние поверхности становятся
расширяемыми.

`A-010 Project Workspace Surface` является конкретной post-V01 workbench
поверхностью внутри этой модели: file tree, file viewer/editor, terminal/PTY,
diff/check entry points and workspace refs. `A-004` отвечает за общий
workbench-shell, commands/contributions and extension points; `A-010` отвечает
за workspace-specific lifecycle, file editing, terminal access, Node/Core
boundary, permissions and traceability.

### Реализованный workspace-centered shell 0.2.6

Web Control Panel использует устойчивую иерархию:

```text
Dashboard
Nodes sidebar
  Node Overview
    Workspace
      Agent
      Workbench
      Jobs
```

`Dashboard` остаётся единственным глобальным product route. Sidebar содержит
только Nodes и Workspaces и скрывается независимо от main surface. `Agent`,
`Workbench` и `Jobs` являются известными placement-scoped surfaces с canonical
routes, а не plugin-controlled layout. Общий Context Inspector монтируется
только при непустом reference stack: без выбранного reference он не резервирует
место, на широком desktop занимает колонку, на узком становится drawer.

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

Так Uprava сохраняет traceability and reviewability даже тогда, когда rich UI
недоступен.

## Уроки VS Code, Atom and Vim

VS Code, Atom and Vim важны не как редакторы кода, а как разные модели
модульности. Для Uprava полезнее не копировать их UI, а взять их
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

Для Uprava это ложится так:

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

Это хорошо подходит Uprava, потому что сохраняет предсказуемость workbench и
при этом позволяет добавлять новые возможности.

### Что берем из Atom

Atom был более hackable package system. Оттуда полезны:

- package как единица расширения;
- services: один package предоставляет capability, другой ее потребляет;
- pane/item model: UI состоит из открываемых items во вкладках/panes;
- высокая локальная расширяемость для power users and internal teams.

Анти-урок Atom: если package может менять DOM/CSS/behavior почти как угодно,
система становится хрупкой, плохо управляемой и тяжелой для поддержки.

Для Uprava стоит взять package/service model, но не брать произвольный
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

Для Uprava это особенно полезно для Go to Cause and context actions:

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

Для Uprava это означает:

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
external plugins еще нет. Сам Uprava должен строиться так, как будто он уже
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

## Plugin Registry и Extension Host

Plugin Registry — не разновидность Tool Registry и не каталог integrations.
Это Core-owned реестр пакетов, которые расширяют саму Uprava через стабильный
Extension API.

```text
Tool Registry
  Какие callable capabilities доступны агенту, человеку или системе?

Plugin Registry
  Какие пакеты установлены и какие contributions они добавляют в Uprava?
```

Plugin может предоставить tools, но это только один contribution type. Plugin
без tools остается полноценным plugin: например theme, renderer, inspector
aspect или набор keyboard commands.

### Что берем у VS Code и Obsidian

У VS Code полезны:

- declarative `contributes` в manifest;
- built-in extensions, которые используют тот же контракт, что и будущие
  внешние extensions;
- commands, views, menus, themes and configuration как именованные extension
  points;
- activation events and context keys;
- extension host отдельно от workbench shell;
- safe mode и возможность отключить проблемный extension.

У Obsidian полезны:

- plugin/theme как понятная пользователю package unit;
- явные install, enable, disable and configure lifecycle;
- local-first управление установленными пакетами;
- themes как first-class plugins, меняющие весь внешний вид приложения через
  контролируемый contract;
- постепенный путь от bundled packages к community/team packages.

Uprava не копирует их authority model буквально. В distributed Uprava Core
остается source of truth для installation, compatibility, permissions and
effective contributions. Web хранит только client/user preferences вроде
выбранной темы и локального layout, а Node исполняет только явно выданные ему
plugin/runtime capabilities.

### Правило развития через first-party plugins

После Plugin Registry v1 новые функциональные направления, которые естественно
являются расширениями work surface, должны по умолчанию поставляться как
bundled first-party plugins. Core/Web/Node base добавляет только общие versioned
contracts, lifecycle, storage, permissions, isolation and fallback, необходимые
целому классу plugins. Конкретные artifact types, renderers, component catalogs,
views and actions принадлежат packages и не получают скрытый privileged path.

Это применимо прежде всего к ближайшим направлениям Visual Artifact System и
Dynamic UI from Agents. Каждый такой slice имеет два равноправных результата:

- полезную пользовательскую функцию через bundled plugin;
- расширение Plugin Registry/Extension Host API, которое может повторно
  использовать следующий first-party или внешний plugin.

Acceptance для функционального plugin slice включает enable/disable,
compatibility, permission-filtered projection, failure isolation and readable
fallback. Bundled package может иметь более высокий trust, но использует те же
manifest and contribution contracts. Так Uprava постепенно движется к модели
Obsidian/VS Code, где сама поставка доказывает extension platform встроенными
расширениями, а базовый shell остаётся небольшим и устойчивым.

Для Dynamic UI plugin-first не означает исполнение plugin или generated
React в main workbench tree. Bundled package регистрирует sandboxed runtime,
Uprava React SDK, layout and action contracts; generated artifact монтируется
в изолированный iframe и общается с host только через типизированный
message/action bridge. Opt-in настройка управляет допуском и capabilities,
но не отменяет isolation boundary.

### Package, installation and contribution

Нужно различать три уровня:

```text
PluginPackage
  immutable package identity, version, manifest, provenance and code/assets

PluginInstallation
  установленная active version, desired state, configuration and grants

PluginContribution
  одно объявленное расширение известного extension point
```

Один package может иметь несколько версий, но installation выбирает ровно одну
active version. Обновление package не должно молча активировать новую version,
если изменились permissions, trust level or compatibility requirements.

Первоначальные install sources:

```text
bundled
  поставляется вместе с согласованным Core/Web/Node release

local
  установлен оператором из локального package artifact, позже

team_catalog
  разрешен policy управляемого deployment, позже

community_catalog
  требует provenance, signing and sandbox policy, позже
```

Plugin Registry v1 реализует только `bundled`. Но persistence и manifest не
должны предполагать, что plugin code всегда скомпилирован в Core или Web.

### Manifest v1

Минимальная форма manifest:

```text
manifest_version
plugin_id
version
display_name
description
publisher
license optional
homepage optional
install_source
trust_level
compatibility:
  core
  web
  node optional
  protocol_versions
activation_conditions
requested_permissions
configuration_schema optional
contributes:
  themes
  commands
  views
  workbench_tabs
  menu_actions
  inspector_aspects
  link_handlers
  block_renderers
  artifact_types
  tools
  workflow_templates
  services
```

`plugin_id` и все contribution ids namespaced and stable. Не-core plugin не
может занять namespaces `core.*` or `uprava.*`. Manifest, configuration schema,
descriptions and contribution counts имеют строгие size/depth limits.

Manifest является data, а не executable program. `activation_conditions` and
`when` clauses используют ограниченную expression grammar над известными
context keys, а не JavaScript.

### Extension points v1 and later

Extension points являются versioned contracts. Plugin не получает общий
`render(anywhere)` или доступ к произвольному React subtree.

```text
ui.theme
  semantic design tokens, Monaco theme and terminal palette

workbench.command
  именованная команда с context, permission and Core authorization

workbench.tab
  вкладка внутри известного typed surface

workbench.menuAction
  action в разрешенном menu/toolbar/context slot

inspector.aspect
  дополнительный aspect для поддерживаемого UpravaRef/entity kind

reference.handler
  preview/open/copy behavior для namespaced reference kind

visual.renderer
  versioned renderer contract with input schema, source matching, scopes and
  mandatory fallback; typed renderer kinds include content, inline fragment,
  viewer enhancement, block and artifact viewer

artifact.type
  metadata and lifecycle contract будущего first-class artifact

agent.tool
  связь package с отдельным Tool Registry definition
```

Registry отклоняет неизвестную major version extension point. Неизвестный
optional contribution может остаться inactive с compatibility diagnostic, но
не должен ломать активацию безопасных независимых contributions того же
package.

### Activation and context keys

Installation и activation не являются одним состоянием.

```text
installed
+ desired enabled
+ compatible package and contribution contracts
+ granted permissions
+ satisfied activation conditions
= effective active contribution
```

Начальные context keys:

```text
client.kind
surface.id
node.id / node.presence
workspace.id / workspace.state
workspace.git.available
session.id / session.state
reference.kind
artifact.type
actor.kind
permission.<id>
```

Context keys являются typed values, опубликованными Core/Web host. Plugin не
может записывать системные keys. Позже plugin может публиковать только свои
namespaced keys через объявленный service contract.

Activation должна быть lazy. Theme metadata можно активировать при bootstrap,
а тяжелый renderer или view module загружается только при совпадении context и
первом обращении к contribution.

Первый shipped executable contribution — bundled trusted plugin
`uprava.markdown` (`0.2.15`). Его `visual.renderer` v1 сопоставляется с
`chat.assistant_message` на `session.timeline`, после чего Web загружает
allowlisted Streamdown adapter. При disabled/incompatible plugin, неизвестном
`implementation_id`, загрузке или render error host показывает исходный текст.
HTML, images и небезопасные URL запрещены adapter-ом; разрешены только
`http`, `https` и `mailto` links.

Content and inline renderer activation обычно начинается не с explicit agent
command, а с source format. Host сначала выбирает content renderer по source
kind and surface, затем применяет зарегистрированные детекторы к точным source
ranges: language-tagged code fence включает syntax highlighter, строгий color
literal — color token renderer, Mermaid/PlantUML fence — diagram renderer.
Source matcher является ограниченной declarative частью contribution; он не
получает возможность выполнить произвольный код до compatibility, trust and
permission checks. Исходный текст остается fallback и не изменяется renderer-ом.

### Core Registry and client Extension Hosts

Plugin architecture состоит не из одного процесса:

```text
Core Plugin Registry
  packages, installations, compatibility, configuration, permissions,
  effective contribution projection, audit

Web Extension Host
  themes, commands, views, tabs, renderers, aspects, link handlers,
  local preference and fallback presentation

Node Plugin Runtime, later and only when required
  local adapters/processes near workspace and credentials
```

Core отдает Web permission-filtered effective contribution snapshot. Web
сопоставляет contribution с известной implementation boundary и не может сам
активировать disabled or incompatible plugin.

Privileged command, даже начавшаяся в plugin UI, снова проходит Core
authorization. Скрытая кнопка не является security boundary: crafted command
от disabled plugin должна быть отклонена Core.

### Trust and execution levels

Trust level и runtime type — разные свойства. Bundled plugin может иметь
высокий trust, но все равно использовать только узкий declarative contract.

```text
data_only
  manifest contributions without executable code; themes are the first case

trusted_bundled
  lazy module shipped with coordinated Uprava release

sandboxed_web
  future Worker/iframe-like runtime with message contract

sandboxed_node
  future isolated local runtime with explicit filesystem/network scopes

external_service
  provider behind Core/Node adapter; never arbitrary code in Web shell
```

Arbitrary third-party JavaScript не исполняется в основном React tree. Plugin
не может monkey-patch DOM, import global CSS, mutate router or read auth/session
credentials. Controlled custom renderers появляются только после отдельного
sandbox and signing design.

### Lifecycle and failure isolation

Базовый lifecycle:

```text
discover package
-> validate manifest, provenance and compatibility
-> install inactive
-> review permissions
-> enable
-> project effective contributions
-> activate lazily by context
-> deactivate
-> disable or update
```

Состояния installation:

```text
disabled
active
incompatible
degraded
error
quarantined later
```

Plugin failure не должен ломать App Shell. Host изолирует renderer/view error,
показывает fallback, записывает bounded diagnostic and позволяет отключить
plugin. Uprava должна иметь safe mode, который запускает только core shell и
явно разрешенные bundled plugins.

Disable не удаляет durable artifacts, refs, tool-call history or configuration.
Исторический объект, чей renderer отключен, продолжает открываться через
metadata/raw fallback.

### Themes as a first-class contribution

Theme меняет UI Uprava глобально, но не является произвольным CSS plugin.

```text
ThemeContribution:
  theme_id
  label
  kind: light | dark | high_contrast
  color_scheme
  semantic_tokens
  monaco_theme
  terminal_palette
```

Theme может задавать только allowlisted semantic tokens:

```text
surface.background
surface.muted
content.primary
content.muted
border.default
border.strong
status.risk
status.notice
focus
selection
editor.*
terminal.*
```

Theme не может задавать selectors, layout, fonts from external origins,
scripts, URLs, `@import` or arbitrary CSS variables. Host проверяет полноту
required tokens, parseability colors and minimum contrast для критических
foreground/background pairs.

Нужно различать:

- installation/enabled state theme plugin — Core-owned;
- выбранная theme — client/user preference;
- effective theme — выбранная доступная theme либо обязательный `core.light`
  fallback.

До появления team/user profile выбранная theme хранится как versioned local
Web preference. Theme bootstrap применяется до первого React render, чтобы не
было light/dark flash, и сверяется с effective contributions после загрузки
Core state.

### Первый plugin: Dark Theme

Первым Plugin Registry v1 package является bundled data-only plugin:

```text
plugin_id: uprava.theme-dark
version: 1.0.0
trust_level: data_only
requested_permissions:
  - ui.theme.contribute
contributes:
  themes:
    - uprava.dark
```

Dark Theme выбрана первой, потому что она:

- сразу доказывает, что plugin меняет саму Uprava, а не только external tool;
- проверяет полный install/enable/select/disable/fallback lifecycle;
- заставляет все first-party surfaces соблюдать semantic design tokens;
- безопасно проверяет Extension Host до появления executable plugins;
- открывает путь local/community themes через data-only package format.

Default остается `core.light`. Установка или enable Dark Theme не меняет
выбранную тему без явного действия пользователя.

Следующим bundled functional plugin разумно сделать `uprava.git-review`: theme
доказывает declarative global UI contribution, а Git Review проверит commands,
workbench tabs, Inspector aspects, permissions and host services.

## Addressable UI

Почти все важное в Uprava должно быть адресуемым:

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

Еще одна важная Vim-inspired идея: Uprava UI должен быть не только
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
куда браузер решил, а Uprava-level cursor по смысловым объектам.

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
  ref: UpravaRef
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
navigation mode = клавиши управляют Uprava UI
input mode = клавиши идут в textarea/editor/terminal
embedded mode = клавиши отданы iframe/webview, Esc возвращает Uprava focus
```

Для plugin blocks это становится частью extension contract. Если plugin хочет
быть first-class частью work surface, он должен описать свои navigable objects
and commands. Если это внешний embed, Uprava может навигироваться только до
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

Так Uprava получает прозрачность без превращения UI в один огромный trace log.
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

Агент может писать Markdown, Mermaid, tables, links. Uprava должен парсить это
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

Полный embed вроде Grafana внутри Uprava лучше не делать базовым способом.
Он быстро тянет auth, iframe security, layout, permissions, чужую навигацию и
непредсказуемый UX.

Более безопасная queued model:

```text
external link
-> rich preview
-> artifact snapshot
-> controlled sandboxed embed only when justified
```

Uprava-native preview часто ценнее, чем iframe чужой системы.

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

Главная цель V01: сам Uprava должен быть написан так, будто он уже
расширяемый, даже если пользовательские plugins еще не подключаются.

### Текущий Web baseline 0.2.6

Workspace-centered shell делает internal extension architecture видимой через
три стабильные workspace surfaces, общий query-addressable Inspector stack,
workspace-aware links и единые status dimensions. Monaco и xterm загружаются
только внутри Workbench; session SSE живёт только в выбранной Agent session, а
Jobs polling — только в активной Jobs surface.

### Feature queue baseline

Пункт `12 Plugin Registry v1` делает modularity visible первым узким slice:

- Core-owned package/install lifecycle;
- manifest-driven Web Extension Host;
- versioned `ui.theme` contribution;
- bundled data-only Dark Theme plugin;
- Plugins/Appearance management UI;
- semantic token, Monaco and xterm theme adapters;
- safe `core.light` fallback and light/dark visual gates.

Bundled Git Review остаётся кандидатом отдельного functional plugin, который
может активировать зарезервированные commands, Workbench tabs, menu actions and
Inspector aspects. Ближайшие плановые platform increments при этом задают
artifact и dynamic UI направления.

Пункты очереди `13 Visual artifact system as plugins` и `14 Dynamic UI from
agents as plugins` продолжают эту линию. Первый активирует artifact types,
renderers/viewers and artifact actions через bundled plugins. Второй добавляет
Generated React runtime, Uprava UI SDK, layout contracts, dynamic renderers and
permissioned action bridge как contributions следующего уровня. Declarative
component catalogs могут остаться fast path для простых blocks, но не
ограничивают expressive model. Оба slice должны оставлять App
Shell работоспособным при disable, incompatibility or failure plugin-а и
сохранять raw/fallback representation.

Перед ними пункт `12b Plugin contribution resolution` и
[`A-012`](012-plugin-contribution-resolution.md) задают общий минимальный
resolver contract. Extension point определяет bounded target и mode
`exclusive` или `ordered`; Host использует детерминированный изменяемый порядок,
а Plugin Panel показывает несколько active contributions с одинаковым
exclusive target как конфликт. Порядок загрузки packages или React modules не
является resolution policy.

### Later

После этих двух plugin-first slices можно добавлять:

- sandboxed custom renderers;
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

`A-007 Agent Tooling, Tool Registry and MCP strategy` отвечает:

```text
Как регистрируются и исполняются callable tools, и как plugin contribution
ссылается на Tool Registry definition без объединения двух registries?
```

## Рабочая формула

Uprava work surface is a stable, addressable, keyboard-navigable, extensible
workbench.

Она не является произвольным page builder. Она состоит из typed surfaces,
blocks, artifacts, references, detail views, aspects, commands, contributions,
services and navigable objects.

Plugins расширяют известные extension points через manifest-driven
contributions. Commands разрешаются из текущего context: addressable object,
selection, permissions and runtime state. Navigation разрешается из текущего
surface, object, selection and mode.

Agents могут производить Markdown/structured output, который Uprava превращает
в blocks, artifacts and references. Go to Source / Cause использует ту же
модель, чтобы превращать любой видимый результат в вход к источнику, evidence
and цепочке причин.
