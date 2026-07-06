# Cortex Feature Queue

Статус: `draft`

Этот документ использует implementation queue вместо phase-based roadmap.

Очередь - это не calendar, milestone ladder or delivery promise. Это
ранжированный набор продуктовых и архитектурных срезов, упорядоченный по
dependency, complexity, risk and value. Позиции могут двигаться по мере
прояснения дизайна.

## Правила очереди

Каждый элемент очереди должен фиксировать:

- **Value** - почему это важно пользователю или Cortex как системе.
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

| Order | Done | Mechanism / Feature Slice | First Useful Slice | Dependency | Complexity |
| --- | --- | --- | --- | --- | --- |
| 0 | + | V01 Distributed Agent Control Panel | Multi-node chat/session control panel | Current design baseline | High |
| 1 | + | Security baseline | Trusted-dev warning, node auth, local web auth, credential handling, audit minimum | V01 control path | High |
| 2 | - | Runtime/session hardening | Robust lifecycle, resume, stop, blocked, stale states | V01 runtime path | Medium |
| 3 | - | Workspace shell and reference model | Stable refs and routes for future workspace evidence | V01 entity/session model | Medium |
| 4 | - | Read-only Project Workspace Inspector | File tree, metadata, safe text viewer | Workspace refs, Node file reads | Medium |
| 5 | - | Workspace intervention layer | Lightweight editor, terminal, command history, diff/check entry points | Read-only inspector, events | High |
| 6 | - | Causality and trace UX | Coarse source/cause links with raw fallback | Workspace refs, event log | Medium |
| 7 | - | Git and review basics | Better diff, branch/worktree awareness, check results | Workspace intervention, trace | Medium |
| 8 | - | Tool Registry v1 | Real tool metadata, permissions, routing and audit policy | V01 capability model, events | High |
| 9 | - | Plugin Registry v1 | Installed plugin metadata, configuration, exposed tools and artifact types | Tool Registry v1 | High |
| 10 | - | First external integrations | Git provider and task tracker integration slices | Tool/Plugin Registry | High |
| 11 | - | Visual artifact system | Test reports, richer diffs, timelines, dashboards/forms as first-class artifacts | Trace, registry contracts | High |
| 12 | - | Dynamic UI from agents | Schema/tool/plugin-rendered UI with safe fallbacks | Visual artifact system, plugins | High |
| 13 | - | Task-based sandbox runtime | Bounded run contract, isolated workspace, expected evidence | Runtime, workspace, trace | Very high |
| 14 | - | Hybrid managed sessions | Persistent session can spawn bounded runs and merge evidence back | Task runtime | Very high |
| 15 | - | Team/cloud model | Users, roles, shared projects, managed Core/nodes | Mature personal workflow | Very high |
| 16 | - | Beyond software development | Research, analytics, documents, finance, knowledge workflows | Mature artifact/plugin model | Very high |

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

**Current implementation note:** `controlled_dev` with `CORTEX_WEB_AUTH=auto`
enables local password setup/login, session and CSRF cookies, protected browser
routes, origin checks, node bearer credentials for heartbeat/control, node
revoke/rotate, private Node state-file permissions where supported, token
redaction and minimal `security_audit_events` records. `local_trusted` remains
available for loopback-only V01 development and keeps the warning banner.

**Target direction:** Дорасти до permissions, secrets handling, stronger audit,
mTLS or request signing, keychain-backed credentials, team RBAC and managed
cloud security без изменения Core/Node responsibility split.

### 2. Runtime/session hardening

**Value:** Делает live agent work надежной, а не похожей на wrapped CLI.

**First useful slice:** Clear lifecycle states, explicit expiry/resume behavior,
blocked approvals, interrupt/stop semantics, stale node handling and degraded
resume messaging.

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

**Target direction:** Shared addressability for UI navigation, agent prompts,
review decisions, plugin blocks and task-run packages.

### 4. Read-only Project Workspace Inspector

**Value:** Дает пользователю увидеть, где работает агент, до добавления прямых
intervention tools.

**First useful slice:** Workspace file tree, file metadata, safe text file
viewer, readable states for large/binary/ignored/generated/permission-denied
files and node-side workspace boundary enforcement.

**Target direction:** Project surface, который позже сможет принять editor,
terminal, diff, checks, artifacts and trace links.

### 5. Workspace intervention layer

**Value:** Дает человеку narrow control, когда прямое действие быстрее, чем
просить агента описать или исправить собственное окружение.

**First useful slice:** Controlled text writes or patch applies, workspace
terminal/PTY or command runner, command/output history, session-level diff and
basic check/test entry points.

**Target direction:** Lightweight developer workbench ergonomics без превращения
в full browser IDE.

### 6. Causality and trace UX

**Value:** Снижает стоимость review, связывая result с evidence без выгрузки raw
logs в пользовательский интерфейс.

**First useful slice:** Coarse links from answers, commands, diffs, checks and
artifacts to source events, with explicit unknown/missing-cause states and raw
fallbacks.

**Target direction:** Более богатый cause graph and trace timeline после
стабилизации event quality and artifact semantics.

### 7. Git and review basics

**Value:** Developer work требует changed-file awareness and review ergonomics.

**First useful slice:** Branch/worktree snapshot, changed-file list, diff view,
check entry points, warning badges for risky workspace state.

**Target direction:** Git provider integration, PR/MR comment import, review
queues, CI follow-up loops and review-ready task outputs.

### 8. Tool Registry v1

**Value:** Tools становятся системными capabilities с permissions, routing,
schemas, UI contracts and audit policy, а не скрытым agent behavior.

**First useful slice:** Core-owned registry for Cortex-native workspace/session
tools and Node capabilities.

**Target direction:** External providers, MCP/native/hybrid adapters, tool call
trace and agent-readable capability discovery.

### 9. Plugin Registry v1

**Value:** Cortex становится extensible без hardcoding каждого tool, block and
integration внутри workbench.

**First useful slice:** Installed plugin metadata, versions, configuration,
requested permissions, exposed tools, artifact types and compatibility.

**Target direction:** Plugin-provided commands, renderers, link handlers,
workflow templates and governed extension surfaces.

### 10. First external integrations

**Value:** Agent work должен подключаться к реальным development systems, не
скрывая integration behavior за текстом.

**First useful slice:** Git provider and Linear/task-tracker slices with visible
objects, actions, trace and permission checks.

**Target direction:** Native, MCP, Node-local, external-provider and hybrid
integration adapters.

### 11. Visual artifact system

**Value:** Results such as diffs, checks, timelines, reports, diagrams and
dashboards должны быть inspectable UI objects, а не только chat text.

**First useful slice:** First-class artifacts for diff/check reports and trace
timeline with source references and fallbacks.

**Target direction:** Artifact gallery, richer visual review, dashboards, UML,
forms and embedded external views.

### 12. Dynamic UI from agents

**Value:** Agents and tools могут возвращать structured interactive surfaces там,
где text имеет неправильную форму.

**First useful slice:** Schema-driven or registered renderer blocks with
sanitized snapshots, source refs, permissions and markdown/table fallback.

**Target direction:** Plugin-rendered blocks, controlled embeds, generated UI
sandboxing and agent-readable UI state.

### 13. Task-based sandbox runtime

**Value:** Cortex может запускать bounded background work with explicit scope,
isolation, evidence and review-ready output.

**First useful slice:** Task contract, isolated workspace/branch, context
package, event log, expected evidence and result package.

**Target direction:** Durable workflow state, queues, CI/webhook wakeups, PR/MR
flow and reproducible review packages.

### 14. Hybrid managed sessions

**Value:** Live sessions and background tasks становятся одним work loop вместо
отдельных продуктов.

**First useful slice:** Persistent session может spawn bounded run and link run
evidence back into session trace/review model.

**Target direction:** Orchestrated workflows, semi-deterministic pipelines,
handoff between live and bounded work and review debt visibility.

### 15. Team/cloud model

**Value:** Cortex расширяется от personal workbench до shared distributed Agent
OS.

**First useful slice:** Multi-user projects, roles, shared node visibility, team
audit trail and managed Core deployment path.

**Target direction:** Managed cloud nodes, node pools, organization-level
plugin/integration governance, stronger secrets model and billing if needed.

### 16. Beyond software development

**Value:** Та же node, agent, tool, artifact, trace and workflow model может
поддерживать broader knowledge work.

**First useful slice:** Выбрать одну non-code vertical только после того, как
developer artifact/plugin model станет достаточно сильной для переноса.

**Target direction:** Research, analytics, documents, presentations, finance,
monitoring and knowledge-base workflows.

## Открытые вопросы очереди

- Насколько строгим должен быть первый security baseline, прежде чем
  рекомендовать non-local node?
- Сколько workspace reference model нужно реализовать до первого read-only
  inspector UI?
- Должен ли first intervention layer начинаться с terminal, editor или
  diff/check?
- Должны ли git/review basics идти до Tool Registry v1, или registry contracts
  нужно посадить раньше, чтобы не получить hardcoded integration path?
- Какая integration лучше как первый proof: GitHub/GitLab, Linear, MCP or
  internal Cortex-native tool set?
- Насколько маленькой может быть первая visual artifact system, чтобы при этом
  уже изменить product experience beyond text?
