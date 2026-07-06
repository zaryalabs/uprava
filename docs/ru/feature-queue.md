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

Это описано в [v01.md](v01.md).

## Обзор очереди

Current release baseline: `0.1.7`. Закрытые пункты `0` through `5`, unified
audit hardening release и `5a` workspace renderer release соответствуют shipped
versions, зафиксированным в [`releases.md`](releases.md).
Следующий плановый пункт очереди - daily-use hardening and deployment readiness
перед добавлением новых продуктовых механизмов.

| Order | Done | Mechanism / Feature Slice | First Useful Slice | Dependency | Complexity |
| --- | --- | --- | --- | --- | --- |
| 0 | + | V01 Distributed Agent Control Panel | Multi-node chat/session control panel | Current design baseline | High |
| 1 | + | Security baseline | Trusted-dev warning, node auth, local web auth, credential handling, audit minimum | V01 control path | High |
| 2 | + | Runtime/session hardening | Robust lifecycle, resume, stop, blocked, stale states | V01 runtime path | Medium |
| 3 | + | Workspace shell and reference model | Stable refs and routes for future workspace evidence | V01 entity/session model | Medium |
| 4 | + | Read-only Project Workspace Inspector | File tree, metadata, safe text viewer | Workspace refs, Node file reads | Medium |
| 5 | + | Workspace intervention layer | Lightweight editor, terminal, command history, diff/check entry points | Read-only inspector, events | High |
| 5a | + | Workspace renderer and PTY terminal layer | Monaco file/diff renderers and xterm-backed interactive PTY sessions | Workspace intervention, Core/Node control channel | High |
| 6 | - | Daily-use hardening and deployment readiness | Stable panel layout, product polish, server deploy path, CI/CD baseline | `0.1.7` workbench, security baseline | High |
| 7 | - | Causality and trace UX | Coarse source/cause links with raw fallback | Workspace refs, event log | Medium |
| 8 | - | Git and review basics | Better diff, branch/worktree awareness, check results | Workspace intervention, trace | Medium |
| 9 | - | Tool Registry v1 | Real tool metadata, permissions, routing and audit policy | V01 capability model, events | High |
| 10 | - | Plugin Registry v1 | Installed plugin metadata, configuration, exposed tools and artifact types | Tool Registry v1 | High |
| 11 | - | First external integrations | Git provider and task tracker integration slices | Tool/Plugin Registry | High |
| 12 | - | Visual artifact system | Test reports, richer diffs, timelines, dashboards/forms as first-class artifacts | Trace, registry contracts | High |
| 13 | - | Dynamic UI from agents | Schema/tool/plugin-rendered UI with safe fallbacks | Visual artifact system, plugins | High |
| 14 | - | Task-based sandbox runtime | Bounded run contract, isolated workspace, expected evidence | Runtime, workspace, trace | Very high |
| 15 | - | Hybrid managed sessions | Persistent session can spawn bounded runs and merge evidence back | Task runtime | Very high |
| 16 | - | Team/cloud model | Users, roles, shared projects, managed Core/nodes | Mature personal workflow | Very high |
| 17 | - | Beyond software development | Research, analytics, documents, finance, knowledge workflows | Mature artifact/plugin model | Very high |
| 18 | - | Audit follow-up refactors | Core/Node module split, generated protocol contracts, async workspace command API | `0.1.6` audit hardening | Medium |

## Детали очереди

### 0. V01 Distributed Agent Control Panel

**Value:** Дает первый осязаемый продукт: пользователь может запустить Core,
подключить одну или несколько nodes, bind projects/workspaces, start persistent
Codex-backed sessions and control those sessions from a web UI.

**First useful slice:** Описан в [v01.md](v01.md).

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
baseline and current Core/Web/Node local profile.

**First useful slice:** Переработать Web Control Panel под длительное
использование: расположение панелей, информационную плотность, навигационный
ритм, переключение workspace/session, terminal/editor/diff ergonomics and
empty/loading/error states. Сделать визуальный design pass, чтобы текущая
функциональность ощущалась связной и пригодной для continuous use. Добавить
реальный server deployment path с documented environment settings,
reverse-proxy/TLS assumptions, persistent volumes, logs, backup/restore
expectations and CI/CD baseline, который запускает quality gates and can deploy
the controlled instance.

**Risk:** Этот срез легко расползается в redesign будущих поверхностей или в
притворство, что продукт уже является multi-user production release. Scope
нужно держать вокруг текущего single-user or controlled deployment, а детальный
checklist уточнять по actual daily use.

**Target direction:** Создать стабильный personal/server operating mode,
которым можно пользоваться постоянно, пока строятся trace, git/review,
registries, plugins, artifacts and task-runtime work.

### 7. Causality and trace UX

**Value:** Снижает стоимость review, связывая result с evidence без выгрузки raw
logs в пользовательский интерфейс.

**First useful slice:** Coarse links from answers, commands, diffs, checks and
artifacts to source events, with explicit unknown/missing-cause states and raw
fallbacks.

**Target direction:** Более богатый cause graph and trace timeline после
стабилизации event quality and artifact semantics.

### 8. Git and review basics

**Value:** Developer work требует changed-file awareness and review ergonomics.

**First useful slice:** Branch/worktree snapshot, changed-file list, diff view,
check entry points, warning badges for risky workspace state.

**Target direction:** Git provider integration, PR/MR comment import, review
queues, CI follow-up loops and review-ready task outputs.

### 9. Tool Registry v1

**Value:** Tools становятся системными capabilities с permissions, routing,
schemas, UI contracts and audit policy, а не скрытым agent behavior.

**First useful slice:** Core-owned registry for Uprava-native workspace/session
tools and Node capabilities.

**Target direction:** External providers, MCP/native/hybrid adapters, tool call
trace and agent-readable capability discovery.

### 10. Plugin Registry v1

**Value:** Uprava становится extensible без hardcoding каждого tool, block and
integration внутри workbench.

**First useful slice:** Installed plugin metadata, versions, configuration,
requested permissions, exposed tools, artifact types and compatibility.

**Target direction:** Plugin-provided commands, renderers, link handlers,
workflow templates and governed extension surfaces.

### 11. First external integrations

**Value:** Agent work должен подключаться к реальным development systems, не
скрывая integration behavior за текстом.

**First useful slice:** Git provider and Linear/task-tracker slices with visible
objects, actions, trace and permission checks.

**Target direction:** Native, MCP, Node-local, external-provider and hybrid
integration adapters.

### 12. Visual artifact system

**Value:** Results such as diffs, checks, timelines, reports, diagrams and
dashboards должны быть inspectable UI objects, а не только chat text.

**First useful slice:** First-class artifacts for diff/check reports and trace
timeline with source references and fallbacks.

**Target direction:** Artifact gallery, richer visual review, dashboards, UML,
forms and embedded external views.

### 13. Dynamic UI from agents

**Value:** Agents and tools могут возвращать structured interactive surfaces там,
где text имеет неправильную форму.

**First useful slice:** Schema-driven or registered renderer blocks with
sanitized snapshots, source refs, permissions and markdown/table fallback.

**Target direction:** Plugin-rendered blocks, controlled embeds, generated UI
sandboxing and agent-readable UI state.

### 14. Task-based sandbox runtime

**Value:** Uprava может запускать bounded background work with explicit scope,
isolation, evidence and review-ready output.

**First useful slice:** Task contract, isolated workspace/branch, context
package, event log, expected evidence and result package.

**Target direction:** Durable workflow state, queues, CI/webhook wakeups, PR/MR
flow and reproducible review packages.

### 15. Hybrid managed sessions

**Value:** Live sessions and background tasks становятся одним work loop вместо
отдельных продуктов.

**First useful slice:** Persistent session может spawn bounded run and link run
evidence back into session trace/review model.

**Target direction:** Orchestrated workflows, semi-deterministic pipelines,
handoff between live and bounded work and review debt visibility.

### 16. Team/cloud model

**Value:** Uprava расширяется от personal workbench до shared distributed Agent
OS.

**First useful slice:** Multi-user projects, roles, shared node visibility, team
audit trail and managed Core deployment path.

**Target direction:** Managed cloud nodes, node pools, organization-level
plugin/integration governance, stronger secrets model and billing if needed.

### 17. Beyond software development

**Value:** Та же node, agent, tool, artifact, trace and workflow model может
поддерживать broader knowledge work.

**First useful slice:** Выбрать одну non-code vertical только после того, как
developer artifact/plugin model станет достаточно сильной для переноса.

**Target direction:** Research, analytics, documents, presentations, finance,
monitoring and knowledge-base workflows.

### 18. Audit follow-up refactors

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
- Должны ли git/review basics идти до Tool Registry v1, или registry contracts
  нужно посадить раньше, чтобы не получить hardcoded integration path?
- Какая integration лучше как первый proof: GitHub/GitLab, Linear, MCP or
  internal Uprava-native tool set?
- Насколько маленькой может быть первая visual artifact system, чтобы при этом
  уже изменить product experience beyond text?
