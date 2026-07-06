# A-007 Plugins, Tool Registry and MCP Strategy

Статус: `working-position`

Этот документ фиксирует рабочую позицию по ключевой механике `A-007 Plugins,
Tool Registry and MCP Strategy`.

Главная позиция: `A-007` - это не только вопрос "где живет Tool Registry" и не
только вопрос "нужен ли MCP gateway". Это механизм, который описывает, **как
Uprava управляет tools, plugins, integrations and runtime dependencies across
Core, Node Daemon, agent runtimes and external providers**.

Ключевое уточнение: проблема MCP в Uprava - это не только protocol routing.
Это dependency/runtime management:

- какие MCP servers, CLI utilities, native adapters and Uprava-native tools
  должны быть доступны;
- на каких нодах, проектах, workspaces, sessions and agents;
- кто устанавливает, запускает, обновляет and останавливает эти зависимости;
- как credentials and secrets материализуются рядом с execution;
- как Core видит actual capabilities and failures;
- как tools становятся traceable, permissioned and visible in UI.

Рабочая позиция:

```text
Core owns desired state, registries, policies, toolsets and trace.
Node Daemon owns reconciliation, local execution and capability reporting.
Runtime providers run concrete dependency classes.
MCP is one tool/integration protocol, not the whole extension model.
```

Для MCP dependencies Uprava должен иметь abstract runtime provider model.
`ToolHive` является основным кандидатом для MCP runtime management, но не
должен быть единственным или обязательным механизмом. Uprava должен уметь
работать с разными providers:

```text
McpRuntimeProvider:
  simple-local
  remote-http
  docker-local
  toolhive-local
  toolhive-kubernetes
```

## Vision

### Какую проблему решает механика

Agent tools сегодня обычно настраиваются локально в конкретном client/runtime:

```text
Codex config
Claude Code config
Cursor MCP config
VS Code / Copilot config
custom scripts
shell PATH
local credentials
```

Это удобно для одного разработчика на одной машине, но плохо масштабируется до
Uprava-модели:

- несколько Node Daemons;
- разные host OS, containers, devboxes and sandboxes;
- разные agent providers;
- project-specific and session-specific toolsets;
- shared team policies;
- visible trace;
- reviewable outputs;
- permissions;
- external integrations;
- local workspace tools;
- future cloud/sandbox runtimes.

Если каждый agent provider самостоятельно читает локальные MCP/CLI configs,
Core теряет контроль над ключевыми вопросами:

- какой tool действительно был доступен агенту;
- какая версия server/binary/schema использовалась;
- на какой Node tool был установлен и запущен;
- кто разрешил tool;
- какие secrets были материализованы;
- как tool call связан с trace, artifact, UI block or external entity;
- как отключить tool для конкретного проекта, ноды или session;
- как понять, почему tool отсутствует или degraded.

С другой стороны, попытка реализовать "свой MCP под каждый Notion/Grafana/GitLab
server" превращает Uprava в бесконечную гонку интеграций. Это не нужно.

Нужна другая модель:

```text
Core describes desired tools/dependencies.
Node Daemon reconciles local reality.
Specialized runtime providers run/manage dependency classes.
Agent sees a scoped tool surface.
Uprava records the call path, policy and output.
```

### Главная модель

Нужно разделить четыре слоя, которые часто смешиваются.

#### Tool

`Tool` - конкретная callable capability с input schema, output schema,
permissions, risk level, routing target and optional renderer/artifact
contract.

Примеры:

- `workspace.read_file`;
- `workspace.run_command`;
- `notion.search_pages`;
- `github.create_issue`;
- `grafana.query_prometheus`;
- `uprava.emit_causality_narrative`.

#### Dependency

`Dependency` - runtime artifact, который нужен, чтобы tool был доступен:
binary, container, MCP server, remote endpoint, native adapter, OAuth
connection, local package, provider runtime.

Примеры:

- `git` binary;
- `rg` binary;
- `gh` CLI;
- `notion-remote` MCP server;
- local `filesystem` MCP server;
- Docker image with MCP server;
- ToolHive workload;
- native GitHub integration adapter;
- Codex runtime adapter.

Один dependency может expose несколько tools. Один tool может иметь разные
execution backends в разных deployment profiles.

#### Plugin / Integration

`Plugin` - package-level extension: tools, UI surfaces, visual renderers,
artifact types, workflow templates, parsers, schemas, permissions and
configuration.

`Integration` - configured connection to an external or local system:
GitHub account, Notion workspace, Grafana instance, local Docker daemon,
Kubernetes cluster, internal service, MCP endpoint.

Plugin and integration могут требовать dependencies, но не равны им.

#### Runtime Provider

`Runtime Provider` - механизм, который умеет materialize and operate
dependencies:

```text
cli provider
  checks/install/runs binaries

container provider
  pulls/builds/runs containers

mcp provider
  starts/proxies MCP servers

toolhive provider
  delegates MCP server lifecycle to ToolHive

native provider
  calls external SaaS API or Core-local implementation
```

Короткая формула:

```text
Tool is what the agent/user can call.
Dependency is what must exist for the tool to work.
Plugin is where capabilities and UI contracts come from.
Runtime provider is who makes the dependency real.
```

### Почему MCP не должен быть единственным механизмом

MCP полезен как protocol для exposing tools to agents. Но Uprava needs more
than MCP:

- Core-owned permissions;
- UI visibility;
- artifacts and visual renderers;
- source/evidence/cause refs;
- workflow integration;
- node placement;
- dependency lifecycle;
- secret materialization;
- versioned trace;
- user-facing configuration;
- agent-provider-independent toolsets.

Поэтому MCP should be one adapter path:

```text
MCP adapter
Native API adapter
CLI adapter
Node-local adapter
Core-native tool
External provider adapter
Hybrid adapter
```

### Почему один Uprava MCP в agent config означает gateway

Если agent config содержит только один MCP server:

```text
agent -> uprava MCP
```

а агент должен через него использовать Notion, Grafana, GitLab, Confluence or
other upstream MCP servers, то Uprava MCP неизбежно становится generic MCP
gateway/proxy:

```text
agent
-> uprava MCP gateway
  -> upstream Notion MCP
  -> upstream Grafana MCP
  -> upstream GitLab MCP
  -> Uprava-native tools
  -> Node-local tools
```

Это не плохо, но это нужно назвать честно.

Граница такая:

```text
Do:
  protocol-level MCP aggregation, filtering, routing, tracing and forwarding

Do not:
  manually reimplement every external MCP server as custom Uprava logic
```

Generic MCP gateway должен уметь:

- `tools/list` upstream discovery;
- namespacing and conflict resolution;
- tool filtering;
- `tools/call` forwarding;
- error mapping;
- timeout/cancellation;
- schema snapshotting;
- permission checks before call;
- trace events around call;
- result wrapping/redaction;
- upstream health reporting.

Он не должен знать доменную семантику Notion or Grafana, если эти systems
подключены как external MCP. Доменная семантика появляется только тогда, когда
Uprava выбирает native adapter path for better UX.

### ToolHive как основной кандидат, а не обязательное ядро

ToolHive важен как validation of the model. Он уже делит MCP platform на
runtime, registry, gateway and portal:

- Runtime запускает MCP servers locally/in containers or in Kubernetes;
- Registry Server помогает curated catalog of MCP servers;
- Virtual MCP Server aggregates multiple backend MCP servers into one endpoint;
- UI/CLI дают operational management;
- есть support для remote MCP, containerized servers, package-manager schemes,
  groups, secrets, audit, tool filtering and optimization.

Для Uprava это не означает "ToolHive становится Core". Более точная модель:

```text
Uprava Core
  desired state, profiles, policy, permissions, trace, UI, artifacts

Node Daemon
  dependency reconciler and capability reporter

ToolHive
  optional provider for MCP runtime management
```

То есть ToolHive может быть `McpRuntimeProvider`, который решает тяжелую часть
MCP lifecycle. Uprava сохраняет собственную модель продукта поверх него.

## Пользовательские сценарии

### 1. Включить Notion на всех developer nodes

Пользователь или admin включает Notion integration в Core:

```text
Enable Notion MCP for node_pool: dev
Expose tools to projects: all
Allowed agents: Codex, Claude Code
Auth: shared OAuth connection or user-scoped OAuth connection
Runtime provider: toolhive-local
```

Core не редактирует руками configs всех agent providers. Он обновляет desired
state. Каждая подходящая Node получает effective profile, reconciler запускает
или обновляет runtime dependency, reports actual state, and Core exposes tools
to allowed sessions.

### 2. Включить Grafana только на одной серверной ноде

Grafana может быть доступна только на ноде, которая имеет network access to
internal observability:

```text
scope:
  node: sre-linux-01
dependency:
  grafana-mcp
routing:
  only sessions placed on sre-linux-01
```

Если session запущена на другой Node, UI должен показать:

```text
Grafana tools unavailable:
  reason: dependency only available on sre-linux-01
  action: move session / request access / use remote provider
```

### 3. Включить GitLab через native adapter, а не MCP

Если Uprava хочет хороший product UX for merge requests, diffs, checks,
review comments and artifacts, native adapter может быть лучше, чем generic
MCP:

```text
GitLab native adapter
  Core owns OAuth, webhooks, domain objects and UI renderers
  Tools are exposed to agents through Tool Registry
  Execution happens through Core/external provider adapter
```

MCP can still exist as compatibility path, but not as source-of-truth for the
integration.

### 4. Дать агенту CLI utility

Для command-line utilities модель проще:

```text
DependencyProfile requires rg, git, cargo
Node verifies or installs binaries
Workspace/session policy decides which commands are callable
Agent invokes typed command tool or terminal/PTY under Node policy
```

Важно не превращать все CLI в raw unrestricted shell. Stable tools should be
typed where possible:

```text
tool: git.status
args: { repo_ref }
execution: node-local
policy: read-only
renderer: git status view
```

Raw shell remains available only through explicit terminal/PTY/run-command
surface with permissions and trace.

### 5. Session-specific MCP toolset

Для sensitive tasks можно создать session-specific profile:

```text
session abc:
  include:
    - notion.search_pages
    - github.list_issues
  exclude:
    - notion.create_page
    - github.merge_pull_request
  lifetime:
    until session end
```

Node or ToolHive may run the same upstream server, but Uprava gateway/toolset
filters what the agent sees and what calls are allowed.

## Agent-facing сценарии

### Direct ToolHive exposure

Быстрый compatibility path:

```text
Agent config:
  uprava MCP
  toolhive vMCP
```

Плюсы:

- быстрее проверить external MCPs;
- меньше Uprava gateway code at first;
- ToolHive already handles aggregation and remote/local MCP runtime.

Минусы:

- часть policy/trace/audit остается outside Uprava;
- сложнее session-specific toolsets;
- UI не всегда знает, какой upstream tool реально был вызван;
- agent provider configs снова содержат больше одного endpoint.

### Uprava-only exposure

Более сильная Uprava product model:

```text
Agent config:
  uprava MCP only

uprava MCP
  -> Uprava-native tools
  -> Node-local CLI/tools
  -> ToolHive vMCP
  -> external MCP servers
  -> native adapters
```

Плюсы:

- один agent-facing endpoint;
- Core sees every tool call;
- better permissions, trace, artifacts, session scoping;
- consistent across Codex/Claude/other providers.

Минусы:

- Uprava must implement generic MCP gateway/forwarding;
- more responsibility for protocol compatibility;
- needs careful failure handling and performance design.

Рабочая позиция: support both paths during exploration, but design the system
so the long-term product-correct path is Uprava-only exposure.

## Architecture

### Responsibility boundaries

```text
Core Backend
  Tool Registry
  Plugin Registry
  Integration Registry
  Dependency Profile Registry
  Toolset/Profile resolution
  Permissions and approval policy
  Secrets refs and account connections
  Routing decisions
  Trace/events/artifact metadata
  Web Control Panel configuration UI

Node Daemon
  Dependency Reconciler
  Capability Reporter
  Agent Provider Adapter host integration
  local CLI/container/MCP runtime control
  ToolHive adapter when enabled
  workspace-local execution
  local health checks
  local secret materialization

Runtime Providers
  install/start/stop/update dependencies
  expose endpoints/process handles
  report health and discovered tools
  enforce runtime-specific isolation where possible

Agent Runtime
  consumes scoped tool surface
  calls tools through provider-native means, MCP, CLI/API or terminal
```

### Control flow

```text
User/admin changes desired state in Core
-> Core computes effective dependency profile per Node/project/session
-> Node receives desired state through control channel
-> Node Reconciler compares desired vs actual
-> Runtime Provider installs/starts/stops dependencies
-> Node discovers capabilities/tools and health
-> Node reports actual state to Core
-> Core updates Tool Registry/Toolset snapshot
-> Agent session receives scoped tools
-> Tool calls are routed and traced through Core/Node/provider
```

### Core registries

#### Tool Registry

Tool Registry stores callable capabilities:

```text
tool_id
display_name
description
input_schema
output_schema
risk_level
permission_scopes
execution_kind
routing_target
source_dependency_ref optional
source_plugin_ref optional
renderer_contract optional
artifact_contract optional
approval_policy
version/schema_hash
```

#### Plugin Registry

Plugin Registry stores package-level extension:

```text
plugin_id
version
origin
provided_tools
provided_dependencies
provided_renderers
provided_artifact_types
provided_workflow_templates
configuration_schema
requested_permissions
compatibility
update_policy
```

#### Integration Registry

Integration Registry stores configured connections:

```text
integration_id
kind: native | mcp | cli | external_provider | hybrid
account_connection_ref optional
endpoint_ref optional
secret_refs
project_bindings
node_bindings
policy
status
```

#### Dependency Profile Registry

Dependency profiles describe desired state:

```yaml
id: default-dev-tools
scope:
  node_pools: [dev]
  projects: [all]
dependencies:
  - id: git
    kind: cli_binary
    provider: system
    version: system
  - id: rg
    kind: cli_binary
    provider: system
    version: ">=14"
  - id: notion
    kind: mcp_server
    provider: toolhive-local
    source: registry:notion-remote
    group: uprava-default
    secrets:
      - ref: notion_oauth
        target: NOTION_TOKEN
    tools:
      allow:
        - search_pages
        - get_page
      deny:
        - delete_page
exposure:
  agents:
    codex:
      mode: uprava-mcp
    claude-code:
      mode: uprava-mcp
```

### Desired vs actual state

Core must not assume that desired state has been applied. Node reports actual
state:

```yaml
node_id: node_local_macbook
profile_version: 42
dependencies:
  - id: notion
    desired: present
    actual: running
    provider: toolhive-local
    endpoint: http://127.0.0.1:4483/mcp
    health: ok
    discovered_tools:
      - notion.search_pages
      - notion.get_page
    schema_hash: sha256:...
  - id: grafana
    desired: present
    actual: failed
    reason: missing_secret
```

UI should always distinguish:

```text
configured
installing
running
available to this session
degraded
failed
blocked by policy
blocked by missing auth
blocked by node capability
```

### Dependency kinds

Initial dependency kind taxonomy:

```text
cli_binary
  git, rg, cargo, gh, docker, uv, node

local_mcp_server
  stdio/SSE/Streamable HTTP server that runs on Node

remote_mcp_server
  external HTTP MCP endpoint proxied or called by provider

containerized_mcp_server
  MCP server packaged as Docker/Podman image

native_adapter
  Uprava-owned integration adapter, usually Core-side or provider-side

uprava_native_tool
  internal Core/Node capability surfaced as tool

agent_provider_adapter
  Codex, Claude Code, OpenCode, etc.
```

This taxonomy is not exhaustive. The important rule is that dependencies are
first-class managed objects, not hidden implementation details inside agent
text or local config files.

### Runtime provider interface

Conceptual provider contract:

```text
plan(desired_dependency, node_context) -> reconciliation_plan
apply(plan) -> operation_id
status(dependency_instance) -> actual_state
discover(dependency_instance) -> capabilities/tools/schemas
stop(dependency_instance)
remove(dependency_instance)
logs(dependency_instance)
health(dependency_instance)
```

For MCP providers:

```text
start_server
proxy_remote_server
create_group/toolset
expose_endpoint
discover_tools
filter_or_namespace_tools
materialize_secrets
report_audit_refs
```

ToolHive adapter can implement this contract using ToolHive CLI/API. A
simple-local provider can implement only a smaller subset.

### ToolHive-backed MCP runtime

ToolHive should be treated as a primary candidate for MCP runtime provider
because it already covers many operational concerns:

- local MCP server execution;
- remote MCP proxying;
- containerized runtime;
- package-manager based server builds;
- groups;
- Virtual MCP Server aggregation;
- registry/catalog;
- secrets;
- tool filtering and name overrides;
- audit logging;
- Kubernetes operator path.

Integration shape:

```text
Core DependencyProfile
-> Node Reconciler
-> ToolHive adapter
-> ToolHive workload/group/vMCP
-> discovered MCP tools
-> Uprava Tool Registry snapshot
```

Important: ToolHive can manage MCP runtime. Uprava still owns:

- product-level permissions;
- project/session scoping;
- trace semantics;
- visual blocks and artifacts;
- integration UI;
- agent provider routing;
- Uprava-native tools.

### Generic MCP gateway

If Uprava exposes only one MCP endpoint to agents, Uprava needs a generic MCP
gateway.

Minimal gateway responsibilities:

```text
list_tools:
  collect from Uprava-native tools
  collect from Node-local tools
  collect from upstream MCP providers
  apply project/session/user/agent policy
  namespace/rename/filter
  return scoped list

call_tool:
  validate tool exists in scoped toolset
  check permission and approval policy
  record tool_call.started
  route to Core/Node/upstream MCP/native provider
  stream or collect result
  redact/wrap result if needed
  record tool_call.completed/failed
```

Gateway must store a tool snapshot per session/run:

```text
session_id
tool_id
upstream_tool_name
upstream_server_id
schema_hash
description_hash
runtime_provider
dependency_instance
policy_version
```

This is required for reproducibility and review. MCP tools can change over
time; trace must know what the agent saw when it made the decision.

### Agent config management

Primary design goal: minimize provider-specific config mutation.

Preferred long-term agent config:

```text
Codex:
  mcp_server: uprava-node-bridge

Claude Code:
  mcp_server: uprava-node-bridge

Other providers:
  equivalent Uprava endpoint/adapter
```

Core should not regularly rewrite every provider's external MCP list. Node
Daemon may perform one-time bootstrap or explicit registration:

```text
install/register Uprava bridge with agent provider
verify config health
avoid overwriting user-owned unrelated entries
```

Provider-specific config management remains necessary for:

- bootstrap;
- compatibility mode;
- direct ToolHive exposure experiments;
- providers that cannot consume Uprava tool surface otherwise.

But it should not be the main control plane.

### Secrets and credentials

Secrets must be first-class and never become plain agent config.

Model:

```text
Core stores account connection and secret refs.
Node receives permission to materialize secret for one dependency instance.
Runtime provider injects secret at process/container/proxy start.
Trace stores secret ref and materialization event, never secret value.
Secret is revoked/expired when dependency/session/profile ends.
```

Credential scopes:

```text
user-scoped
project-scoped
team-scoped
node-scoped
session-scoped
provider-managed
```

For OAuth-based external systems, Core may own account connection and token
refresh, while Node gets short-lived materialized credentials only when local
runtime needs them.

### Permissions and security

MCP servers are not harmless metadata. They are executable tool surfaces and
often code dependencies. Security model must cover:

- supply chain trust;
- package/image provenance;
- version pinning;
- filesystem access;
- network access;
- secret injection;
- tool description/tool metadata poisoning;
- overbroad tool lists;
- cross-tool data forwarding;
- output redaction;
- per-tool approvals;
- audit and trace;
- deactivation/revocation.

Tool filtering must not be treated as the only security boundary. If a tool is
hidden from `tools/list`, calls to that tool must also be denied at routing
time unless an internal workflow explicitly uses it under policy.

Risk levels:

```text
read-only
workspace-write
external-read
external-write
credentialed-action
destructive
privileged-local
network-broad
```

Approval policy can depend on:

```text
user
project
agent
node
tool
args
target external entity
risk level
runtime mode
```

### Events

Important events for trace and UI:

```text
dependency_profile.updated
dependency.reconcile.started
dependency.install.started
dependency.install.completed
dependency.install.failed
dependency.start.started
dependency.start.completed
dependency.start.failed
dependency.health.changed
dependency.discovered
dependency.removed

secret.materialization.requested
secret.materialization.granted
secret.materialization.failed
secret.expired

toolset.resolved
tool.registry.updated
tool.snapshot.created
tool.call.started
tool.call.forwarded
tool.call.output_available
tool.call.completed
tool.call.failed

mcp.upstream.connected
mcp.upstream.disconnected
mcp.tools.changed
mcp.gateway.policy_denied
```

### Storage implications

Likely Core storage areas:

```text
plugins
plugin_versions
integrations
integration_accounts
tools
tool_versions
dependency_profiles
dependency_profile_assignments
dependency_instances
node_capabilities
node_dependency_status
session_toolsets
tool_snapshots
tool_call_events
secret_refs
secret_materialization_events
```

Node-local storage can keep:

```text
last desired profile version
actual dependency cache
provider-specific instance ids
local health cache
local logs pointers
runtime socket/port metadata
```

Core remains source-of-truth for desired state. Node-local state is actual
state and operational cache.

## Native vs MCP vs CLI decision rules

### Use Uprava-native tool when

- tool is part of Uprava itself;
- it modifies sessions, artifacts, UI, refs, event log, workspace state or
  permissions;
- it needs tight integration with trace and visual semantics;
- it is a safe internal control-plane action.

Examples:

- create artifact;
- emit causality narrative;
- inspect workspace ref;
- resolve UI selection;
- start/stop session;
- request approval.

### Use CLI adapter when

- capability is naturally local to workspace;
- mature CLI already exists;
- output can be parsed or shown as command output;
- execution must happen near files/env/runtime.

Examples:

- `git`;
- `rg`;
- `cargo`;
- `npm test`;
- `gh` when local auth/workspace context is useful.

### Use external MCP when

- a good MCP server already exists;
- generic tool access is enough;
- domain UX is not yet worth native investment;
- we want fast integration;
- server can be safely isolated and governed;
- output can remain text/resource/tool-result oriented.

Examples:

- early Notion search/read;
- early Grafana query;
- experimental internal tools;
- external SaaS capabilities where MCP server is maintained upstream.

### Use native adapter when

- Uprava needs high-quality product UX;
- integration has important domain objects;
- we need webhooks, pagination, conflict handling, rich permissions or durable
  snapshots;
- outputs become first-class artifacts/visual blocks;
- generic MCP hides too much behavior.

Examples:

- GitHub/GitLab PR review flow;
- Linear issue/project workflow;
- Notion/Confluence document artifacts if document UX becomes core;
- Grafana/observability dashboards if visual analysis becomes core.

### Use ToolHive-backed MCP runtime when

- dependency is MCP server;
- local/containerized/remote MCP lifecycle matters;
- we need groups, vMCP, isolation, secrets, audit or Kubernetes path;
- we want to avoid building MCP server operations from scratch.

### Use simple MCP runtime when

- early MVP/spike;
- single local server;
- no fleet management;
- no complex secret or isolation requirements;
- direct HTTP remote endpoint is enough.

## Queue strategy

### V01 constraints

V01 should not become an integration platform before the developer
workbench works. Minimal obligations:

- keep Tool Registry and Plugin Registry schemas compatible with additional tools;
- model Node capabilities and dependency status;
- expose Uprava-native workspace/session tools;
- avoid hardcoding agent-provider-specific configs as the primary control
  plane;
- leave room for Dependency Profiles and MCP runtime providers.

### Feature queue target

Feature queue should make modularity real:

- Dependency Profile Registry;
- Node Dependency Reconciler;
- first MCP runtime provider;
- generic MCP gateway or bridge spike;
- ToolHive-backed provider spike;
- basic UI for tool/dependency status;
- basic permissions and approvals;
- tool call trace;
- plugin-provided renderer/artifact metadata.

### Spike recommendation

The first practical spike should answer:

```text
Can Core desired state cause a Node to run Notion MCP through a provider,
discover tools, expose them to an agent, and trace a call?
```

Recommended spike path:

```text
Core profile:
  notion via toolhive-local

Node:
  ToolHive adapter
  group/workload/vMCP setup
  discovery status back to Core

Agent:
  try direct ToolHive vMCP
  try Uprava MCP forwarding

Trace:
  record tool list snapshot and call event
```

The point of the spike is not to choose ToolHive forever. The point is to test
whether provider-backed MCP runtime management solves the daemon dependency
problem with acceptable complexity.

## Relationship with other design docs

### Relationship with A-001 Distributed Architecture

A-007 depends on the Core / Node Daemon split:

```text
Core: desired state, policy, registry, trace
Node: local runtime, dependency reconciliation, actual capabilities
```

Tools must not make clients bypass Core and talk directly to every node or
external provider without trace and permissions.

### Relationship with A-002 Run Mode

Run Mode decides how an agent session/task is executed. A-007 decides which
tools/dependencies are available inside that run and how they are exposed.

Different runtime strategies may require different dependency profiles:

```text
persistent local runtime -> node-local tools and local MCP
ephemeral sandbox -> container image/tools profile
remote provider runtime -> remote MCP/native tools only
hybrid runtime -> split profile
```

### Relationship with A-003 Distributed Runtime Coordination

A-003 dispatches work to nodes and handles node/resource state. A-007 adds
dependency capability and tool availability as placement constraints:

```text
session needs grafana.internal
-> only nodes with dependency available and network access can run it
```

### Relationship with A-004 Modular UI and Work Surface

Plugins can contribute panels, blocks, commands and configuration surfaces.
A-007 provides the registry and permission model behind those UI extensions.

### Relationship with A-005 Dynamic UI from Agents

Dynamic UI from agents often comes from tool/plugin output. A-007 must connect:

```text
tool output schema
-> renderer/artifact contract
-> permissions
-> trace refs
```

### Relationship with A-006 Visual Rendering and Artifact Semantics

A-006 defines how visual objects behave. A-007 defines how tools/plugins
register renderers, artifact types and output contracts.

### Relationship with A-008 Go to Source and Causality UX

Tool calls and dependency operations are part of the cause/evidence graph:

```text
artifact -> tool call -> upstream MCP server -> dependency instance -> profile
```

### Relationship with A-009 Human-Agent Dual Interface

A-009 primary agent-facing contract for Uprava Core should remain CLI/API and
shared refs/commands. MCP is an adapter path for exposing tools to external
agent providers, not the only internal control interface.

### Relationship with A-010 Project Workspace Surface

Workspace-local tools and CLI dependencies must respect workspace boundaries,
filesystem permissions, PTY/session policies and project-scoped trace.

## Open questions

- Should Uprava implement generic MCP gateway in Core, Node, or split between
  Core policy and Node-local proxy?
- How much of ToolHive should be controlled through CLI vs API in the first
  spike?
- Is direct ToolHive vMCP acceptable as a temporary exposure mode, or should
  every agent-visible call go through Uprava from the start?
- How do we represent upstream MCP tools that change schemas during a long
  persistent session?
- Should Tool Registry store every upstream MCP tool as first-class tool, or
  store only session-scoped snapshots until user promotes them?
- What is the minimal safe install policy for package-manager MCP servers
  (`npx`, `uvx`, `go`)?
- How should user-owned local agent configs be protected from Uprava bootstrap
  changes?
- Which integrations deserve native adapters early rather than MCP path?
- What is the exact boundary between plugin package, integration connection,
  dependency instance and tool snapshot in storage?

## Quality questions

For every tool/integration/dependency:

- Is it clear whether this is a tool, dependency, plugin, integration or
  provider?
- Where is desired state stored?
- Where does actual execution happen?
- Which Node or provider owns runtime lifecycle?
- How are versions and schemas pinned or snapshotted?
- What secrets are required and where are they materialized?
- What tool calls are visible to Core trace?
- What is shown in UI if the dependency is missing, failed or unauthorized?
- Can the tool be disabled for one project/session/node without uninstalling
  everything globally?
- Is the tool exposed to agents through direct provider config, Uprava MCP,
  CLI/API or another adapter?
- Is there a safe fallback if ToolHive or another runtime provider is absent?
- Does the output have renderer/artifact/source/cause semantics where needed?

## References

- [Model Context Protocol architecture](https://modelcontextprotocol.io/docs/learn/architecture)
- [MCP tools specification](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
- [ToolHive documentation](https://docs.stacklok.com/toolhive)
- [ToolHive Virtual MCP Server](https://docs.stacklok.com/toolhive/guides-vmcp/)
- [ToolHive run MCP servers](https://docs.stacklok.com/toolhive/guides-cli/run-mcp-servers)
