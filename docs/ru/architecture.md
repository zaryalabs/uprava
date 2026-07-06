# Uprava Architecture

Статус: `active`

Этот документ фиксирует первую архитектурную позицию по client/server модели Uprava.

## Короткое решение

Uprava должен иметь отдельный **Core Backend** как control plane. Клиенты работают через Core, а работа с конкретными машинами, файлами, терминалами, процессами, sandbox и AI-agent lifecycle выполняется через **Node Daemons**.

Важно: Core - обязательная архитектурная абстракция, но не обязательно удаленный SaaS backend в каждом deployment. В локальном режиме Core может запускаться на той же машине, что UI и Node Daemon.

```text
Clients
web / desktop / mobile / CLI
        |
        v
Core Backend / Control Plane
API, web control panel, auth, discovery, registry, workflows, events, artifacts
        |
        v
Node Daemons / Data Plane
files, terminal, processes, sandboxes, agent lifecycle, local capabilities
        |
        v
AI Agents / Tools / Workspaces
persistent sessions, task runs, hybrid flows
```

## Термины

### Core Backend

Центральный backend и control plane Uprava.

Core отвечает за глобальную модель системы: проекты, пользователей, права, ноды, capabilities, agent sessions, agent runs, workflows, artifacts, event log, trace, tool registry, routing и web control panel.

Core не должен становиться процессом, который сам напрямую работает с файловыми системами всех машин. Работа с конкретным окружением должна оставаться на стороне Node Daemon.

### Control Panel

Web UI, который разворачивается рядом с Core Backend и дает доступ к управлению Uprava из браузера.

На раннем этапе это может быть основной клиент. Позже рядом могут появиться desktop, mobile and CLI clients.

### Client

Пользовательский интерфейс к Core.

Типы клиентов:

- web;
- desktop;
- mobile;
- CLI.

Клиент не должен быть обязан напрямую подключаться к каждой ноде. Базовая модель: клиент общается с Core, Core маршрутизирует команды, события и состояние между клиентом и нодами.

### Node

Зарегистрированное вычислительное окружение, где может выполняться работа.

Node может быть:

- локальным компьютером;
- сервером;
- devbox;
- cloud workspace;
- sandbox;
- microVM host;
- будущей managed cloud node.

Термин `host` можно использовать как техническое пояснение, но продуктовая сущность лучше называется `Node`, потому что Uprava ближе к distributed/cloud модели, чем к простому списку машин.

### Node Daemon

Системный демон, запущенный на Node.

Это не AI-агент. Node Daemon - инфраструктурный процесс, который:

- регистрирует Node в Core;
- сообщает capabilities;
- запускает и останавливает AI agents;
- управляет persistent agent sessions;
- выполняет task-based runs в sandbox/workspace;
- дает доступ к файлам;
- открывает terminal/PTY или command execution;
- стримит логи, events and outputs;
- применяет изменения;
- запускает checks/tests;
- управляет локальными workspace, env, credentials and runtime limits.

Node Daemon - основной data plane Uprava.

### AI Agent

AI-agent workload, который запускается через Node Daemon или подключается как внешний provider.

AI Agent может работать в разных execution modes:

- persistent agent session;
- task-based sandbox run;
- hybrid managed session.

## Почему Core Backend нужен

### Discovery

Node Daemon должен регистрироваться в одном месте:

```text
node id
status / heartbeat
available projects
available agents
available tools
runtime capabilities
sandbox capabilities
security limits
```

Без Core каждый клиент должен был бы сам искать ноды, держать соединения, понимать capabilities and синхронизировать состояние. Это быстро ломается на mobile, distributed and team scenarios.

### Mobile and web access

Телефон или браузер не должны напрямую подключаться к ноутбуку, devbox or sandbox. Им нужен стабильный endpoint.

Core дает этот endpoint и позволяет:

- открыть web UI;
- посмотреть состояние задач;
- подключиться к agent session;
- прочитать trace;
- посмотреть diff/artifacts;
- принять review decision;
- остановить or продолжить работу.

### Workflow state

Task-based режим, hybrid mode, CI callbacks and long-running work требуют долговечного состояния:

- что было запущено;
- где работа остановилась;
- какие checks прошли;
- какой webhook пришел;
- какой следующий step;
- кто должен принять решение.

Это состояние должно жить в Core, а не в конкретном клиенте.

### Trace and event log

Traceability должна быть общей для всех клиентов и нод. Core хранит event log, trace metadata, artifacts metadata, review decisions and workflow state.

Node Daemon может хранить локальные raw logs или большие файлы, но Core должен знать, что существует, где лежит, кто имеет доступ и как это связано с workflow.

### Security and permissions

Core должен быть местом, где принимаются и проверяются системные решения:

- кто видит project;
- кто видит Node;
- кто может открыть terminal;
- кто может запускать agent run;
- кто может использовать tool;
- кто может читать artifact;
- кто может принять diff;
- кто может остановить or удалить session.

Node Daemon должен enforce локальные ограничения, но policy и routing должны быть согласованы через Core.

## Tool Registry

Tool Registry должен жить в Core.

Причина: tools являются частью общей системы capabilities, permissions, UI, trace and routing. Если registry будет только на нодах или клиентах, Core не сможет нормально отвечать на вопросы:

- какие tools доступны в проекте;
- какие tools доступны на конкретной Node;
- какие tools разрешены конкретному пользователю or агенту;
- какие tools можно показать в UI;
- как tool отображается как visual block or artifact;
- какие вызовы tools нужно трассировать;
- куда маршрутизировать tool call;
- какие schemas, permissions and risk levels у tool.

При этом выполнение tool не обязано происходить в Core.

Модель:

```text
Core Tool Registry
metadata, schema, permissions, routing, UI contract, audit policy

Node Tool Runtime
local execution, files, terminal, local env, local credentials

External Tool Provider
MCP, SaaS API, GitHub, Linear, Grafana, Docker, MLflow, etc.
```

Core знает, что tool существует и как с ним работать. Node Daemon или external provider выполняют конкретное действие там, где находятся данные, credentials and runtime.

## Plugins and Integrations

Плагины и интеграции - один из главных механизмов модульности Uprava.

Uprava не должен реализовывать все внешние системы сам. Вместо этого Core должен иметь расширяемую модель, через которую можно подключать:

- task trackers: Linear, Jira, GitHub Issues;
- knowledge and docs systems: Notion, Obsidian-like repos, Google Docs;
- git providers: GitHub, GitLab;
- observability and dashboards: Grafana, LangSmith, Langfuse, OpenTelemetry, Phoenix;
- runtimes and infrastructure: Docker, sandbox providers, devboxes, Kubernetes-like environments;
- ML/experiment systems: MLflow and similar tools;
- custom internal company tools;
- MCP servers.

### Plugin Registry

Plugin Registry должен жить в Core рядом с Tool Registry.

Plugin Registry отвечает за:

- installed plugins;
- plugin versions;
- plugin configuration;
- exposed tools;
- visual blocks;
- artifact types;
- workflow templates;
- permissions requested by plugin;
- integration accounts/connections;
- compatibility with Core and Node Daemon versions.

Tool Registry отвечает за конкретные callable capabilities. Plugin Registry отвечает за package-level extension: откуда tool пришел, какие UI/artifact/workflow extensions он добавил, как он конфигурируется and обновляется.

### Integration adapters

Интеграция может подключаться разными способами:

- **MCP adapter** - если внешняя система уже доступна через MCP или MCP хорошо подходит как tool protocol.
- **Native API adapter** - если нужен контроль над auth, pagination, webhooks, rate limits, domain objects or visual UX.
- **Node-local adapter** - если tool должен выполняться рядом с файлами, терминалом, локальными credentials or runtime.
- **External provider adapter** - если tool исполняется во внешнем SaaS/provider.
- **Hybrid adapter** - metadata and permissions живут в Core, execution идет через Node or external provider.

MCP важен, но не должен быть единственным способом интеграции. Для Uprava важно не только вызвать tool, но и:

- показать его в UI;
- трассировать вызовы;
- связать результат с artifact/workflow;
- применить permissions;
- поддержать review;
- встроить визуализации;
- сделать результат понятным человеку and агенту.

### Integration contract

Каждая интеграция должна описывать:

```text
identity
capabilities
tool schemas
auth requirements
permission scopes
risk level
routing target
execution location
event/audit policy
artifact types
visual blocks
workflow hooks
```

Это нужно, чтобы интеграции были first-class частью системы, а не набором скрытых API calls за текстовым ответом агента.

## Разделение ответственности

### Core Backend отвечает за

- API for clients;
- web control panel;
- auth and user/session management;
- projects;
- Node registry and discovery;
- Node capabilities;
- agent session/run registry;
- workflow state;
- event log;
- trace metadata;
- artifact metadata;
- Tool Registry;
- Plugin Registry;
- integration registry and configuration;
- permissions and policies;
- routing commands to Node Daemons;
- webhooks from GitHub/GitLab/Linear/CI;
- review queue and decisions;
- global dashboards.

### Node Daemon отвечает за

- Node registration and heartbeat;
- local capability probing;
- workspace management;
- file access;
- terminal/PTY;
- process lifecycle;
- persistent agent sessions;
- task-based sandbox runs;
- sandbox/microVM integration;
- local tool execution;
- local logs and output streaming;
- checks/tests execution;
- local resource limits;
- local secret/env access;
- applying patches and file changes.

### Client отвечает за

- human interaction;
- visualization;
- review UX;
- command initiation;
- session attach/detach;
- artifact browsing;
- mobile/desktop/web ergonomics.

Client should not own durable workflow state.

### AI Agent отвечает за

- reasoning;
- tool use within granted scope;
- producing changes and artifacts;
- reporting expected evidence;
- exposing unresolved risks;
- following mode-specific contract.

## Connection model

Базовая безопасная модель: Node Daemon сам устанавливает outbound connection к Core.

Это упрощает:

- NAT/firewall scenarios;
- подключение личного компьютера;
- временные devboxes;
- cloud nodes;
- mobile/web access.

Core затем маршрутизирует команды и streams через это соединение.

Прямое client-to-node подключение можно рассматривать позже как optimization для локального режима, но не как обязательную базовую архитектуру.

## Deployment profiles

### Local single-user

```text
same machine:
Core Backend + Web Control Panel + Node Daemon
```

Подходит для раннего MVP и локальной разработки.

### Personal distributed

```text
server/VPS/cloud:
Core Backend + Web Control Panel

personal machines/devboxes:
Node Daemons

clients:
web/mobile/desktop/CLI
```

Подходит для сценария "работаю с компьютера и телефона, агенты бегут на разных машинах".

### Team/cloud

```text
managed Core Backend
multiple users
multiple projects
multiple Node Daemons
shared workflows
role-based access
```

Подходит для коммерческого/team продукта.

## Открытые вопросы

- Окончательно ли называем сущность `Node`, а не `Host`?
- Где хранить большие artifacts: в Core storage, на Node или во внешнем object storage?
- Какие secrets можно хранить в Core, а какие должны оставаться только на Node?
- Как описывать tool capabilities: через MCP schema, собственный contract или adapter model?
- Где граница между plugin, integration, tool and visual block?
- Какие интеграции стоит делать через MCP, а какие требуют native adapter?
- Как versioning tools/plugins влияет на воспроизводимость trace?
- Должен ли Core уметь выполнять lightweight tools сам, или любой execution должен идти через Node/Provider?
- Какой минимальный протокол нужен между Core and Node Daemon для MVP?
- Какой transport выбрать сначала: HTTP polling, WebSocket, gRPC, message queue?
- Как изолировать команды terminal/filesystem в persistent session mode?
- Как показывать пользователю разницу между Core-level tool и Node-local tool?

## Текущая позиция

На текущем уровне vision наиболее сильная архитектурная позиция такая:

- `Core Backend` обязателен как control plane.
- `Web Control Panel` можно сразу разворачивать вместе с Core.
- `Node Daemon` является системным агентом на ноде и основным data plane.
- `AI Agents` являются workloads, а не инфраструктурными демонами.
- `Tool Registry` живет в Core.
- `Plugin Registry` живет в Core рядом с Tool Registry.
- Внешние интеграции подключаются через adapters: MCP, native API, Node-local, external provider or hybrid.
- MCP важен как протокол интеграции, но не должен быть единственным механизмом расширения.
- Tool execution может происходить на Node, во внешнем provider или позднее в самом Core для безопасных lightweight tools.
- Clients должны работать через Core, а не напрямую владеть distributed state.
