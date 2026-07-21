# Uprava Feature Queue

Статус: `active`

Этот документ использует implementation queue вместо phase-based roadmap.

Очередь - это не calendar, milestone ladder or delivery promise. Это
ранжированный набор продуктовых и архитектурных срезов, упорядоченный по
dependency, complexity, risk and value. Позиции могут двигаться по мере
прояснения дизайна.

## Правила очереди

Каждый элемент очереди должен фиксировать:

- **Value** - почему это важно пользователю или Uprava как системе.
- **Dependency** - что должно существовать раньше.
- **Complexity** - сложность реализации and surface area.
- **Risk** - unknowns, security concerns or product ambiguity.
- **First useful slice** - минимальная версия, которую стоит строить.
- **Target direction** - как механизм должен расти без overfitting под первую
  реализацию.

Используйте этот документ, чтобы отвечать на вопрос:

```text
Что строить следующим, и почему это раньше другого?
```

Не используйте его для ответа на вопрос:

```text
Что входит в первую версию?
```

Это описано в разделе [V01](product-evolution.md#v01).

## Обзор очереди

Current release baseline: `0.2.15`. Закрытые пункты `0` through `12a`, unified
audit hardening release и `5a` workspace renderer release соответствуют shipped
versions, зафиксированным в [`releases.md`](../releases.md). Пункт `6` включает
workbench alignment, первый стабильный self-hosted deployment path и
workspace-centered UI follow-up `0.2.6` и Causality/Trace/Deduction slice
`0.2.7`. Runtime boundary refactor зафиксирован implementation baseline
`0.2.9`. Git and review basics зафиксирован implementation baseline `0.2.10`,
Agent Tooling and Tool Registry v1 — `0.2.11`, Plugin Registry v1 — `0.2.12`,
CI/SQLite reliability fix slice — `0.2.13`, отдельная ToolHive Compose topology
для ручного Linear acceptance — `0.2.14`, bundled Markdown renderer plugin —
`0.2.15`.
Следующий плановый пункт очереди — `13 Visual artifact system as plugins`.

| Order | Done | Mechanism / Feature Slice | First Useful Slice | Dependency | Complexity |
| --- | --- | --- | --- | --- | --- |
| 0 | + | V01 Distributed Agent Control Panel | Multi-node chat/session control panel | Current design baseline | High |
| 1 | + | Security baseline | Trusted-dev warning, node auth, local web auth, credential handling, audit minimum | V01 control path | High |
| 2 | + | Runtime/session hardening | Robust lifecycle, resume, stop, blocked, stale states | V01 runtime path | Medium |
| 3 | + | Workspace shell and reference model | Stable refs and routes for future workspace evidence | V01 entity/session model | Medium |
| 4 | + | Read-only Project Workspace Inspector | File tree, metadata, safe text viewer | Workspace refs, Node file reads | Medium |
| 5 | + | Workspace intervention layer | Lightweight editor, terminal, command history, diff/check entry points | Read-only inspector, events | High |
| 5a | + | Workspace renderer and PTY terminal layer | Monaco file/diff renderers and xterm-backed interactive PTY sessions | Workspace intervention, Core/Node control channel | High |
| 6 | + | Daily-use hardening and deployment readiness | Stable panel layout, product polish, server deploy path, CI/CD baseline | `0.1.8` deployable workbench, security baseline | High |
| 7 | + | Отложенные сообщения в сессии | Долговечные одноразовые будущие turn существующей сессии | Runtime/session guards, Core-owned persistence | Medium |
| 8 | + | Background Jobs и scheduled agent runs | Долговечные определения unattended agent work, расписания и наблюдаемые runs | Placements, provider runtime, durable events | High |
| 9 | + | Causality and trace UX | Coarse source/cause links with raw fallback | Workspace refs, event log | Medium |
| 10 | + | Git and review basics | Better diff, branch/worktree awareness, check results | Workspace intervention, trace | Medium |
| 11 | + | Agent Tooling and Tool Registry v1 | Uprava MCP, progressive discovery, ToolHive runtime, scoped registry and trace | V01 capability model, events | High |
| 12 | + | Plugin Registry v1 | Core registry, manifest-driven Web Extension Host and bundled Dark Theme plugin | Stable workbench shell, design tokens | High |
| 12a | + | Markdown renderer plugin | Typed `visual.renderer` contribution, safe Streamdown rendering and plain-text fallback for assistant chat content | Plugin Registry v1 | Medium |
| 13 | - | Visual artifact system as plugins | Plugin-driven content enhancements for code, colors and diagrams plus artifact viewers for reports, diffs and timelines | Trace, Plugin Registry v1 | High |
| 14 | - | Dynamic UI from agents as plugins | Opt-in bundled Generated React plugin with sandboxed runtime, Uprava UI SDK, safe fallbacks and permissioned actions | Plugin-delivered visual artifact system | High |
| 15 | - | Task-based sandbox runtime | Bounded run contract, isolated workspace, expected evidence | Runtime, workspace, trace | Very high |
| 15a | - | Provider-native persistent execution policy | Safe provider defaults, explicit unsafe mode, real approvals and visible effective policy | Task-based sandbox runtime, provider-native persistent runtime | Very high |
| 16 | - | Hybrid managed sessions | Persistent session can spawn bounded runs and merge evidence back | Task runtime | Very high |
| 17 | - | Team/cloud model | Users, roles, shared projects, managed Core/nodes | Mature personal workflow | Very high |
| 18 | - | Beyond software development | Research, analytics, documents, finance, knowledge workflows | Mature artifact/plugin model | Very high |
| 19 | - | Audit follow-up refactors | Core/Node module split, generated protocol contracts, async workspace command API | `0.1.6` audit hardening | Medium |

## Детали очереди

### 0. V01 Distributed Agent Control Panel

**Value:** Дает первый осязаемый продукт: пользователь может запустить Core,
подключить одну или несколько nodes, bind projects/workspaces, start persistent
Codex-backed sessions and control those sessions from a web UI.

**First useful slice:** Описан в разделе [V01](product-evolution.md#v01).

**Target direction:** Сохранить первый продукт маленьким, но не закрывать system
model для workspaces, providers, tools, plugins, visual artifacts, task runs,
mobile and team/cloud modes.

### 1. Security baseline

**Value:** Делает V01 control path достаточно безопасным для использования за
пределами полностью trusted local prototype, не притворяясь full team/cloud
security.

**First useful slice:** Explicit deployment profiles, visible non-production
warning until hardened mode is enabled, node enrollment/auth, credential storage
rules, revoke/rotate basics, local web auth/session handling, origin/CSRF checks
where relevant, token redaction and minimal security/audit events.

**Current implementation note:** `controlled_dev` with `UPRAVA_WEB_AUTH=auto`
is the supported V01 profile. It enables local password setup/login, session
and CSRF cookies, protected browser routes, origin checks, node bearer
credentials for heartbeat/control, node revoke/rotate, private Node state-file
permissions where supported, token redaction and minimal
`security_audit_events` records. `local_trusted`, disabled browser auth and
auto-approved enrollment are rejected at startup.

**Target direction:** Дорасти до permissions, secrets handling, stronger audit,
mTLS or request signing, keychain-backed credentials, team RBAC and managed
cloud security без изменения Core/Node responsibility split.

### 2. Runtime/session hardening

**Value:** Делает live agent work надежной, а не похожей на wrapped CLI.

**First useful slice:** Clear lifecycle states, explicit expiry/resume behavior,
blocked approvals, interrupt/stop semantics, stale node handling and degraded
resume messaging.

**Current implementation note:** Core and Node now persist and project
start/ready/running/blocked/resuming/stopped/error/expired runtime state,
bounded provider resume refs, idle expiry, stale/offline/revoked node warnings,
detached-session gates, approval request/resolution state and command preflight.
The Web Control Panel and agent projection only advertise send-turn and
approval-resolution commands when those commands match Core runtime/session
preflight, and resolved historical approval blocks no longer expose approval
actions.

**Target direction:** Поддержать несколько runtime strategies and provider
adapters без изменения Core/UI concepts.

### 3. Workspace shell and reference model

**Value:** Позволяет будущим chat, trace, artifacts, review and agents
ссылаться на одну и ту же workspace evidence, не затаскивая full inspector в
V01.

**First useful slice:** Stable ids, routes and reference shapes for project,
workspace, session, turn, message, runtime event and reserved future workspace
objects such as file, file range, edit, terminal session, command, output range,
diff hunk, check result, artifact and trace event.

**Current implementation note:** Shared Rust and Web protocol contracts now
define stable Uprava refs for project, placement, workspace, session, runtime,
turn, message, block, artifact, event, command, approval, warning, tool call,
file/file range, terminal/command/output range, diff hunk, check result,
workspace edit, trace event, external entity and unknown future refs. Web
Control Panel has stable project, workspace, placement, node and session route
helpers, a project route, a workspace route alias, inspector stack URL encoding
and explicit fallback handling for reserved future workspace refs.

**Target direction:** Shared addressability for UI navigation, agent prompts,
review decisions, plugin blocks and task-run packages.

### 4. Read-only Project Workspace Inspector

**Value:** Дает пользователю увидеть, где работает агент, до добавления прямых
intervention tools.

**First useful slice:** Workspace file tree, file metadata, safe text file
viewer, readable states for large/binary/ignored/generated/permission-denied
files and node-side workspace boundary enforcement.

**Current implementation note:** Core exposes authenticated placement workspace
tree and file-read routes, dispatching read-only commands to the Node Daemon and
waiting for typed command results. Node Daemon normalizes relative paths,
enforces workspace and allowed-root boundaries, avoids symlink traversal, caps
tree and text reads, and returns explicit states for large, binary, generated,
ignored, missing, symlink and permission-denied paths. Web Control Panel mounts
file tree and safe text viewer on workspace routes.

**Target direction:** Project surface, который позже сможет принять editor,
terminal, diff, checks, artifacts and trace links.

### 5. Workspace intervention layer

**Value:** Дает человеку narrow control, когда прямое действие быстрее, чем
просить агента описать или исправить собственное окружение.

**First useful slice:** Controlled text writes or patch applies, workspace
terminal/PTY or command runner, command/output history, session-level diff and
basic check/test entry points.

**Current implementation note:** Первый intervention slice расширяет Project
Workspace Inspector явным save для text files, bounded workspace command runner,
отображением command/check results, persisted command result history и git diff
snapshot entry point. Core routes these actions через placement-scoped commands
and persists command-result payloads; Node enforces allowed workspace roots,
path normalization, protected generated/ignored paths, text-size caps, no-shell
command execution, timeout limits and bounded output. Web Control Panel exposes
save, `make l`, `make c`, custom command, diff and history controls в workspace
surface.

**Target direction:** Lightweight developer workbench ergonomics без превращения
в full browser IDE.

### 5a. Workspace renderer and PTY terminal layer

**Value:** Делает workspace surface похожей на настоящий developer workbench:
Monaco рендерит code and diffs, а xterm рендерит interactive PTY вместо
имитации terminal через command-runner output.

**First useful slice:** Monaco-backed file editor and diff viewer; Core APIs для
terminal open/list/stream/input/resize/close; Node Daemon PTY lifecycle scoped
to the validated workspace; xterm terminal tabs with attach, resize, input,
output, status and close handling. Bounded command runner остается отдельным
механизмом для traceable controlled checks.

**Current implementation note:** `0.1.7` добавляет shared protocol contracts для
workspace terminal commands and stream frames, Core routes all terminal traffic
через node control channel and WebSocket client stream, Node owns PTY creation
and cleanup внутри workspace cwd, а Web uses Monaco plus xterm.js as
first-class renderers.

**Target direction:** Добавить durable replay endpoints, terminal output refs,
search/copy ergonomics, review decorations, selection/range actions and richer
diff/review workflows без ослабления Core/Node authority boundary.

### 6. Daily-use hardening and deployment readiness

**Value:** Core workbench path уже достаточно полезен, чтобы следующий срез
сделал его удобным и надежным для постоянной работы, а не только
feature-complete в отдельных flows.

**Dependency:** `0.1.7` workspace renderer/PTY baseline, controlled-dev security
baseline, current Core/Web dev profile and host Node run path.

**First useful slice:** Переработать Web Control Panel под длительное
использование: расположение панелей, информационную плотность, навигационный
ритм, переключение workspace/session, terminal/editor/diff ergonomics and
empty/loading/error states. Сделать визуальный design pass, чтобы текущая
функциональность ощущалась связной и пригодной для continuous use. Добавить
реальный server deployment path с documented environment settings,
reverse-proxy/TLS assumptions, persistent volumes, logs, backup/restore
expectations and CI/CD baseline, который запускает quality gates and can deploy
the controlled instance.

**Current implementation note:** `0.2.1` Zarya Web Control Panel alignment
принес flat work-sheet shell, system overview, phased agent-work surface,
workspace/session chrome и visual regression coverage. Релизы `0.2.2` и
`0.2.3` завершили deployable server path: automatic immutable delivery из
`main`, bounded CI workspaces, явные
`prepare -> build -> deploy -> finalize` gates, root-owned deployment inputs,
state-neutral ordinary deploys, production health/SHA/Node finalization и
bounded release retention. Environment controlled instance, TLS/reverse-proxy
assumptions, persistent paths, logging и backup/restore operations описаны в
deployment и CI/CD guides.

UI follow-up `0.2.6` заменил глобальные Nodes/Jobs экраны на workspace-centered
навигацию: sidebar хранит `Nodes -> Workspaces`, workspace содержит известные
поверхности `Agent / Workbench / Jobs`, а Context Inspector появляется только
для выбранного reference. Workbench использует IDE-like file/editor/terminal
композицию, сохраняя Core/Node authority и lazy Monaco/xterm loading.

**Risk:** Этот срез легко расползается в redesign будущих поверхностей или в
притворство, что продукт уже является multi-user production release. Scope
нужно держать вокруг текущего single-user or controlled deployment, а детальный
checklist уточнять по actual daily use.

**Target direction:** Создать стабильный personal/server operating mode,
которым можно пользоваться постоянно, пока строятся trace, git/review,
registries, plugins, artifacts and task-runtime work.

### 7. Отложенные сообщения в сессии

**Value:** Позволяет человеку подготовить follow-up turn, не прерывая активного
агента и не удерживая browser открытым. Отложенное сообщение — один будущий
turn конкретной существующей сессии, а не повторяющаяся автоматизация и не
Job Run.

**Dependency:** Runtime/session admission guards и durable Core persistence;
фактическая отправка должна проходить обычным send-turn path.

**First useful slice:** Core-owned records с explicit timezone, lifecycle
`scheduled -> sending -> sent | failed | cancelled`, список внутри сессии,
edit/reschedule/send-now/cancel, пока запись остаётся `scheduled`. В назначенное
время Core проверяет обычные session/runtime guards. Если turn не принят,
запись остаётся видимой с typed reason и явным действием retry или reschedule,
а не повторяется скрытно.

**Delivered в `0.2.4`:** Core хранит запись и запускает durable dispatcher.
Перед отправкой он атомарно claim'ит запись, вызывает обычный send-turn
admission path и сохраняет typed failure для ручного retry или reschedule. UI
сессии поддерживает создание, edit/reschedule, send-now, cancel и retry.

**Target direction:** Delivery policies вроде exact-time или
not-before-when-ready, видимая history и notifications о failure. Recurrence,
запуск новой сессии, обход approvals и цепочки автоматизации остаются за
пределами этого среза.

### 8. Background Jobs и scheduled agent runs

**Value:** Добавляет управляемый unattended-work mode для повторяемой bounded
agent work, не объявляя бессмертный process или непрозрачный workflow graph
продуктовой моделью.

**Dependency:** Project/workspace placements, обычный provider runtime path и
durable Core events. Job не вводит отдельный скрытый executor. Provider-native
sandboxing и более строгая execution policy из пункта `15a` не блокируют этот
срез: для текущего controlled deployment сознательно принимается изоляция
отдельным OS user и/или VM вместе с рисками unrestricted provider execution.

**First useful slice:** Paused-by-default Job definition с одним target
placement, prompt/task description и параметрами запуска, manual test run и
простыми interval/daily/weekly schedules с explicit IANA timezone. Job работает
только в текущем placement workspace; worktree и isolated task runtime
отложены. Каждый запуск сохраняется как наблюдаемый Job Run. UI показывает
конфигурацию, run history, итоговый summary, доступный provider output/logs,
typed skipped/failed outcomes и переход к обычной session/trace evidence.
Default overlap policy — `skip`, не больше одного active run на Job.

Расписание по умолчанию использует stop-on-error policy: failed или не
стартовавший из-за runtime/admission error run приостанавливает дальнейшие
автоматические запуски Job до явного действия человека. Это opt-out параметр:
пользователь может разрешить расписанию продолжаться после ошибки. Manual run
остаётся доступен независимо от паузы расписания.

Перед автоматическим и обычным interactive start Core по возможности проверяет
provider usage limits. Если Codex сообщает, что у пятичасового или недельного
лимита осталось `5%` или меньше, новый chat/session и Job Run не запускаются с
typed reason. Пользователь может сделать explicit force start. Если provider не
даёт надёжных machine-readable данных, состояние quota должно быть `unknown`, а
не выдуманным числом; отсутствие данных само по себе не блокирует запуск.

**Target direction:** Immutable configuration revisions, event и task-tracker
triggers, explicit buffering policies, budgets, notifications, richer
summaries/evidence, review/PR loops, worktrees и затем isolated task runtimes.
Первый срез исключает visual workflow canvas, arbitrary multi-step pipelines и
unlimited backfill. Ограничения должны добавляться по подтверждённой
необходимости; основная task behavior пока задаётся prompt/description.

**Delivered в `0.2.5`:** Core хранит paused Job definitions и Job Runs со
snapshot конфигурации, атомарно claim-ит interval/daily/weekly IANA schedule
occurrences, использует обычный placement/session/runtime path, показывает
typed overlap/failure outcomes, по умолчанию ставит schedule на pause после
ошибки и применяет общую quota admission с audited force override. Web Control
Panel показывает конфигурацию Job/run, history, summary, ссылки на session
evidence и schedule controls. Codex quota честно остаётся `unknown`, когда CLI
не даёт стабильного machine-readable usage source.

**UI follow-up в `0.2.6`:** Jobs показываются и создаются внутри текущего
workspace. Web фильтрует глобальный Core `/jobs` read endpoint по
`project_placement_id`; Core API и scheduler semantics не менялись. Legacy Job
и Job Run links разрешаются в nested workspace routes с ownership guards.

### 9. Causality and trace UX

**Реализовано в `0.2.7`:** Core хранит типизированные workspace causality
events и отдает глобальный cursor-based event log, coarse
`SessionTraceProjection`, raw event detail и permission-aware ref resolver.
Web показывает trace steps, aspect-based Inspector и фильтруемый raw fallback.
Явный Deduction запускается на Node отдельным ephemeral/read-only provider
process, получает bounded evidence snapshot, проходит Core validation по схеме
и allowlist refs, поддерживает cancel и может быть сохранён как versioned
`CausalityNarrative`.

**Value:** Снижает стоимость review, связывая result с evidence без выгрузки raw
logs в пользовательский интерфейс.

**First useful slice:** Coarse links from answers, commands, diffs, checks and
artifacts to source events, with explicit unknown/missing-cause states and raw
fallbacks.

**Target direction:** Более богатый cause graph and trace timeline после
стабилизации event quality and artifact semantics.

### 10. Git and review basics

**Value:** Developer work требует changed-file awareness and review ergonomics.

**First useful slice:** Branch/worktree snapshot, changed-file list, diff view,
check entry points, warning badges for risky workspace state.

**Current implementation note:** `0.2.10` добавляет Node-owned porcelain-v2 Git
snapshot, persisted Placement git facts, same-repo/branch coordination warning,
changed-file scopes `all / staged / unstaged`, bounded per-file Monaco diff с
binary/raw fallback и resolvable diff/hunk refs. Workbench Review запускает
`make l` и `make c` через существующий async bounded command path, показывает
progress/cancel и долговечную типизированную историю check results.

**Target direction:** Git provider integration, PR/MR comment import, review
queues, CI follow-up loops and review-ready task outputs.

### 11. Agent Tooling and Tool Registry v1

Рабочий план реализации:
[`0.2.11-agent-tooling-tool-registry.md`](../tmp-plans/0.2.11-agent-tooling-tool-registry.md).

**Value:** Агент как first-class citizen получает единый machine interface к
Uprava и внешним integrations, а tools становятся системными capabilities с
permissions, Node/project/session scope, routing, schemas, trace and audit
policy вместо скрытого agent behavior.

**First useful slice:** Core-owned registry для managed tools и observed Node
capabilities; Uprava MCP как основной agent-facing interface; обязательный
progressive discovery `Search -> Inspect -> Execute`; ToolHive-backed runtime с
одним реальным внешним MCP server; effective availability, permissions,
routing and end-to-end tool-call trace.

Uprava не передаёт модели полный каталог schemas. Core/host индексирует upstream
`tools/list`, применяет policy и возвращает через Search только имена и краткие
описания. Inspect раскрывает полную схему одного выбранного tool. Execute заново
проверяет schema, permission and availability перед routing.

Provider-native и Node-local инструменты (`bash`, file tools, `git`, `gh`,
`glab`) не оборачиваются и не проксируются без отдельной продуктовой причины.
Uprava сообщает их version, health and safe authentication status как observed
capabilities, а агент вызывает native CLI напрямую.

**Current implementation note:** `0.2.11` добавляет Core-owned scoped registry,
permission-first `Search -> Inspect -> Execute`, Streamable HTTP Uprava MCP с
краткоживущими session leases, Node inventory и desired/actual reconciliation,
pinned отдельный Compose ToolHive bridge к официальному Linear MCP, Web connect/reconnect/
disconnect и redacted tool-call trace. Linear authorization URL существует
только в эфемерном ответе текущему Web-клиенту; OAuth callback, discovery и
read-only execution готовы к ручной opt-in приёмке, но ещё не подтверждены.

**Target direction:** Более богатый MCP catalog, dynamic server selection,
programmatic tool calling/code mode, дополнительные runtime providers,
approval policies and first-class integration UX. Отдельный Uprava CLI
добавляется только при подтверждённых shell-composition, streaming or batch
сценариях.

### 12. Plugin Registry v1

Рабочий план реализации:
[`0.2.12-plugin-registry-dark-theme.md`](../tmp-plans/0.2.12-plugin-registry-dark-theme.md).

**Value:** Uprava становится extensible без hardcoding каждого tool, block and
integration внутри workbench. Plugin Registry расширяет саму Uprava и не
является разновидностью Tool Registry или integration catalog.

**First useful slice:** Core-owned packages/installations, versioned manifest,
compatibility, configuration and permissions; permission-filtered contribution
projection; Web Extension Host с first-class `ui.theme` contribution; bundled
data-only `uprava.theme-dark`, который можно enable, выбрать, disable и безопасно
заменить на `core.light` без reload or broken UI.

Theme меняет только allowlisted semantic tokens, Monaco theme and terminal
palette. Plugin не получает arbitrary CSS, DOM access or JavaScript execution в
main React tree. Первый slice также приводит first-party UI к theme-safe tokens
и добавляет light/dark visual and contrast gates.

**Current implementation note:** `0.2.12` добавляет отдельные protocol,
persistence and application boundaries Plugin Registry, migration 13,
идемпотентный bundled-package bootstrap, enable/disable and compatibility
lifecycle, permission-filtered effective projection, Plugins/Appearance UI и
versioned preference с безопасным light fallback. `uprava.theme-dark@1.0.0`
является data-only package; arbitrary CSS and executable plugin code в этот
срез не входят.

**Target direction:** VS Code/Obsidian-like package lifecycle, local/team
installation, signed catalogs, activation/context keys, plugin-provided
commands, Workbench views/tabs, Inspector aspects, renderers, link handlers,
artifact types, workflow templates, services and governed sandboxed extension
surfaces. Следующие функциональные направления должны расширять Uprava как
bundled first-party plugins через те же versioned contracts, которые позже
будут доступны внешним plugins. После data-only theme следующими доказательствами
платформы становятся artifact plugins и dynamic UI plugin; Git Review остаётся
кандидатом отдельного functional bundled plugin.

### 13. Visual artifact system as plugins

**Value:** Results such as diffs, checks, timelines, reports, diagrams and
dashboards должны быть inspectable UI objects, а не только chat text.

**Delivery rule:** Пользовательская функциональность поставляется как один или
несколько bundled first-party plugins поверх Plugin Registry and Web Extension
Host. Базовая система владеет generic artifact identity, storage, refs,
permissions, contribution validation, renderer isolation and fallback, но не
hardcode-ит каждый artifact type или его UI. Bundled plugins используют тот же
versioned extension contract, который предназначен для будущих local/team/
community plugins.

**Content pickup rule:** Обычный путь не требует просить агента выдать
специальный artifact descriptor или заранее выбранный UI. Агент пишет обычный
Markdown и использует естественный формат данных; Extension Host подхватывает
его зарегистрированными content/inline renderers. Например fenced code block
получает syntax highlighting, строгий color literal становится активным color
token, а Mermaid/PlantUML fence — diagram preview. Сохраненный текст и его
source range остаются source-of-truth и обязательным fallback. Такое
обогащение само по себе не создает durable artifact; pin/save/export или
review-valued tool result могут отдельно превратить visual object в artifact.

**First useful slice:** Generic visual/artifact contract плюс bundled content
and artifact plugins для Markdown/code rendering, color tokens,
Mermaid/PlantUML diagrams, diff/check reports and trace timeline with source
references, viewers and readable fallbacks. Plugin можно disable или сделать
incompatible без поломки App Shell и без потери доступа к исходному тексту,
raw metadata or evidence.

**Plugin platform increment:** Активировать manifest contributions для
content/inline renderers and source matchers, `artifact_types`,
`block_renderers`, artifact viewers, commands/actions and related context keys;
провести их через Core-owned lifecycle, compatibility, permissions and
effective projection. Acceptance требует одновременно полезной visual/artifact
функции и переиспользуемого контракта, на котором следующий plugin может
подхватить новый source format или добавить свой artifact type без изменения
базового Web shell.

**Target direction:** Artifact gallery, richer visual review, dashboards, UML,
forms and embedded external views, предоставляемые first-party и внешними
plugins. Новые artifact families должны преимущественно добавляться packages,
а generic artifact kernel и Extension Host оставаться небольшими и стабильными.

### 14. Dynamic UI from agents as plugins

**Value:** Agents and tools могут возвращать structured interactive surfaces там,
где text имеет неправильную форму.

**Delivery rule:** Dynamic UI реализуется как bundled first-party plugin поверх
artifact и renderer contracts пункта `13`, а не как привилегированная ветка
основного React tree. Базовая система владеет validation, persistence,
permissions, command/event routing, sandbox boundary and fallback. Plugin
предоставляет versioned Generated React runtime, Uprava React SDK, design
tokens, layout contract, renderers and UI-specific contributions через
общие extension points. Generated code не монтируется в main React
tree даже в trusted mode.

**First useful slice:** Выключенный по умолчанию bundled Generated React
UI plugin. Агент создает версионируемый React/TypeScript artifact,
который проходит controlled build/validation pipeline и исполняется в
sandboxed iframe с жестким CSP/capability policy. Plugin предоставляет
Uprava UI SDK, responsive layout primitives, permissioned action bridge,
persisted state, sanitized/static snapshot and markdown/table/raw fallback. Его
disable, version mismatch, build или render failure оставляют reviewable
artifact and fallback вместо broken surface.

**Plugin platform increment:** Добавить versioned contributions для
generated UI runtimes, React SDK/API compatibility, dynamic renderers, layout
intents, permissioned action bridge, plugin-owned configuration/context keys
and sandbox capabilities. Acceptance требует, чтобы другой plugin мог
переиспользовать эти contracts для нового generated UI family без
изменения App Shell или обхода Core authorization. Declarative component
schema может остаться опциональным fast path для простых forms/cards,
но не является главным expressive model.

**Target direction:** Plugin-rendered blocks, Generated React artifacts,
controlled embeds, capability-scoped runtimes and agent-readable UI state.
Каждый новый dynamic UI family должен
одновременно улучшать пользовательскую функцию и приближать package lifecycle,
extension points, isolation and interoperability к уровню Obsidian/VS Code.

### 15. Task-based sandbox runtime

**Value:** Uprava может запускать bounded background work with explicit scope,
isolation, evidence and review-ready output.

**First useful slice:** Task contract, isolated workspace/branch, context
package, event log, expected evidence and result package.

**Target direction:** Durable workflow state, queues, CI/webhook wakeups, PR/MR
flow and reproducible review packages.

### 15a. Provider-native persistent execution policy

**Value:** Делает persistent provider execution безопасной и понятной, не
подменяя provider sandbox workspace allow-list или Unix account.

**Dependency:** Task-based sandbox runtime и provider-native persistent runtime
path, способный остановиться для policy and approval decisions.

**First useful slice and exit criteria:** Обязательны все четыре условия:

1. sandboxed execution является safe default;
2. unrestricted execution доступна только через explicit unsafe-mode switch;
3. provider approval requests проходят реальный Core/User/Node approval flow
   до продолжения execution;
4. effective sandbox and approval policy видна до start и в runtime
   trace/evidence.

**Accepted risk before delivery:** Audit finding P0-3 остаётся accepted risk
для controlled deployment. Release quality-foundation 0.2.0 не меняет
существующие Codex launch flags, не называет текущие normalized approval events
реальным enforcement и не обещает team, cloud or hostile-workload isolation.

**Target direction:** Применить тот же explicit policy contract к будущим
provider-native persistent runtimes, сохраняя provider-specific enforcement and
evidence.

### 16. Hybrid managed sessions

**Value:** Live sessions and background tasks становятся одним work loop вместо
отдельных продуктов.

**First useful slice:** Persistent session может spawn bounded run and link run
evidence back into session trace/review model.

**Target direction:** Orchestrated workflows, semi-deterministic pipelines,
handoff between live and bounded work and review debt visibility.

### 17. Team/cloud model

**Value:** Uprava расширяется от personal workbench до shared distributed Agent
OS.

**First useful slice:** Multi-user projects, roles, shared node visibility, team
audit trail and managed Core deployment path.

**Target direction:** Managed cloud nodes, node pools, organization-level
plugin/integration governance, stronger secrets model and billing if needed.

### 18. Beyond software development

**Value:** Та же node, agent, tool, artifact, trace and workflow model может
поддерживать broader knowledge work.

**First useful slice:** Выбрать одну non-code vertical только после того, как
developer artifact/plugin model станет достаточно сильной для переноса.

**Target direction:** Research, analytics, documents, presentations, finance,
monitoring and knowledge-base workflows.

### 19. Audit follow-up refactors

**Value:** Сохраняет `0.1.6` audit fixes reviewable, contract-backed and ready
for longer-running tools, не смешивая broad mechanical work с behavior
hardening release.

**First useful slice:** Разделить Core command/event/session code и Node
state/command-runner code into focused modules under current public interfaces;
add generated or schema-checked web protocol contracts; design async workspace
command API для команд, которые перерастают bounded synchronous execution.

**Target direction:** Сделать command lifecycle, session projection, Node state
store, workspace command execution and web protocol shapes independently
testable до того, как Tool Registry and external integrations увеличат surface
area.

## Открытые вопросы очереди

- Насколько строгим должен быть первый security baseline, прежде чем
  рекомендовать non-local node?
- Какие daily-use hardening items обязательны до первого continuously used
  server deployment, а что может подождать следующих feature slices?
- Какой реальный external MCP server лучше взять для ToolHive-backed acceptance
  scenario после обязательного Uprava-native proof?
- Насколько маленькой может быть первая visual artifact system, чтобы при этом
  уже изменить product experience beyond text?
