# Uprava Architecture

Status: `active`

This document records the first architectural position on the Uprava
client/server model.

## Short Decision

Uprava should have a separate **Core Backend** as the control plane. Clients work
through Core, while work with concrete machines, files, terminals, processes,
sandboxes, and the AI-agent lifecycle is performed through **Node Daemons**.

Important: Core is a required architectural abstraction, but it does not have to
be a remote SaaS backend in every deployment. In local mode, Core can run on the
same machine as the UI and Node Daemon.

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

## Terms

### Core Backend

The central backend and control plane of Uprava.

Core owns the global system model: projects, users, permissions, nodes,
capabilities, agent sessions, agent runs, workflows, artifacts, event log,
trace, tool registry, routing, and web control panel.

Core should not become the process that directly works with file systems across
all machines. Work with a concrete environment should stay on the Node Daemon
side.

### Control Panel

The Web UI deployed next to Core Backend that provides browser access to Uprava
management.

At the early stage, this can be the main client. Later, desktop, mobile, and CLI
clients can appear next to it.

### Client

A user interface to Core.

Client types:

- web;
- desktop;
- mobile;
- CLI.

A client should not be required to connect directly to every node. The base
model is: the client talks to Core, and Core routes commands, events, and state
between the client and nodes.

### Node

A registered compute environment where work can run.

A Node can be:

- a local computer;
- a server;
- a devbox;
- a cloud workspace;
- a sandbox;
- a microVM host;
- a future managed cloud node.

The term `host` can be used as a technical explanation, but the product entity
is better called `Node`, because Uprava is closer to a distributed/cloud model
than to a simple list of machines.

### Node Daemon

A system daemon running on a Node.

It is not an AI agent. Node Daemon is an infrastructure process that:

- registers the Node in Core;
- reports capabilities;
- starts and stops AI agents;
- manages persistent agent sessions;
- performs task-based runs in a sandbox/workspace;
- provides file access;
- opens terminal/PTY or command execution;
- streams logs, events, and outputs;
- applies changes;
- runs checks/tests;
- manages local workspaces, environment, credentials, and runtime limits.

Node Daemon is the main data plane of Uprava.

### AI Agent

An AI-agent workload that runs through Node Daemon or connects as an external
provider.

An AI Agent can work in different execution modes:

- persistent agent session;
- task-based sandbox run;
- hybrid managed session.

## Why Core Backend Is Needed

### Discovery

Node Daemon should register in one place:

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

Without Core, each client would have to discover nodes, keep connections,
understand capabilities, and synchronize state on its own. This quickly breaks
in mobile, distributed, and team scenarios.

### Mobile and Web Access

A phone or browser should not connect directly to a laptop, devbox, or sandbox.
They need a stable endpoint.

Core provides this endpoint and allows the user to:

- open the web UI;
- inspect task state;
- attach to an agent session;
- read trace;
- inspect diff/artifacts;
- make a review decision;
- stop or continue work.

### Workflow State

Task-based mode, hybrid mode, CI callbacks, and long-running work need durable
state:

- what was started;
- where work stopped;
- which checks passed;
- which webhook arrived;
- what the next step is;
- who must make a decision.

This state should live in Core, not in a specific client.

### Trace and Event Log

Traceability should be shared across all clients and nodes. Core stores the
event log, trace metadata, artifact metadata, review decisions, and workflow
state.

Node Daemon can store local raw logs or large files, but Core should know what
exists, where it is, who has access, and how it relates to the workflow.

### Security and Permissions

Core should be the place where system decisions are made and checked:

- who can see a project;
- who can see a Node;
- who can open a terminal;
- who can start an agent run;
- who can use a tool;
- who can read an artifact;
- who can accept a diff;
- who can stop or delete a session.

Node Daemon should enforce local limits, but policy and routing should be
coordinated through Core.

## Tool Registry

Tool Registry should live in Core.

Reason: tools are part of the common system of capabilities, permissions, UI,
trace, and routing. If the registry exists only on nodes or clients, Core cannot
properly answer:

- which tools are available in a project;
- which tools are available on a specific Node;
- which tools are allowed for a specific user or agent;
- which tools can be shown in UI;
- how a tool appears as a visual block or artifact;
- which tool calls need tracing;
- where to route a tool call;
- which schemas, permissions, and risk levels the tool has.

At the same time, tool execution does not have to happen in Core.

Model:

```text
Core Tool Registry
metadata, schema, permissions, routing, UI contract, audit policy

Node Tool Runtime
local execution, files, terminal, local env, local credentials

External Tool Provider
MCP, SaaS API, GitHub, Linear, Grafana, Docker, MLflow, etc.
```

Core knows that a tool exists and how to work with it. Node Daemon or an
external provider performs the concrete action where data, credentials, and
runtime live.

## Plugins and Integrations

Plugins and integrations are one of the main modularity mechanisms in Uprava.

Uprava should not implement every external system itself. Instead, Core should
have an extensible model for connecting:

- task trackers: Linear, Jira, GitHub Issues;
- knowledge and docs systems: Notion, Obsidian-like repos, Google Docs;
- git providers: GitHub, GitLab;
- observability and dashboards: Grafana, LangSmith, Langfuse, OpenTelemetry,
  Phoenix;
- runtimes and infrastructure: Docker, sandbox providers, devboxes,
  Kubernetes-like environments;
- ML/experiment systems: MLflow and similar tools;
- custom internal company tools;
- MCP servers.

### Plugin Registry

Plugin Registry should live in Core next to Tool Registry.

Plugin Registry owns:

- installed plugins;
- plugin versions;
- plugin configuration;
- exposed tools;
- visual blocks;
- artifact types;
- workflow templates;
- permissions requested by a plugin;
- integration accounts/connections;
- compatibility with Core and Node Daemon versions.

Tool Registry owns concrete callable capabilities. Plugin Registry owns the
package-level extension: where a tool came from, which UI/artifact/workflow
extensions it added, how it is configured, and how it is updated.

### Integration Adapters

An integration can connect in different ways:

- **MCP adapter** - when the external system is already available through MCP or
  MCP fits well as a tool protocol.
- **Native API adapter** - when Uprava needs control over auth, pagination,
  webhooks, rate limits, domain objects, or visual UX.
- **Node-local adapter** - when a tool must run next to files, terminal, local
  credentials, or runtime.
- **External provider adapter** - when a tool executes in an external
  SaaS/provider.
- **Hybrid adapter** - metadata and permissions live in Core, while execution
  goes through Node or an external provider.

MCP matters, but it should not be the only integration method. For Uprava, it is
important not only to call a tool, but also to:

- show it in UI;
- trace calls;
- connect the result to an artifact/workflow;
- apply permissions;
- support review;
- embed visualizations;
- make the result understandable to humans and agents.

### Integration Contract

Each integration should describe:

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

This is needed so integrations become first-class parts of the system, not a set
of hidden API calls behind the agent's text answer.

## Responsibility Split

### Core Backend Owns

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

### Node Daemon Owns

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

### Client Owns

- human interaction;
- visualization;
- review UX;
- command initiation;
- session attach/detach;
- artifact browsing;
- mobile/desktop/web ergonomics.

Client should not own durable workflow state.

### AI Agent Owns

- reasoning;
- tool use within granted scope;
- producing changes and artifacts;
- reporting expected evidence;
- exposing unresolved risks;
- following the mode-specific contract.

## 0.2.0 Quality Foundation Contracts

### Project, Placement, And Workspace Identity

- A `Project` is a Core-owned logical aggregate. Its identity is independent
  of any Node or local path.
- A `ProjectPlacement` is the physical binding of one Node and one canonical
  local workspace path. Core persistence must enforce uniqueness of
  `(node_id, canonical_workspace_path)`.
- A `Workspace` is the user-facing workbench over a Placement, not another
  persisted identity. The Core resource route is `/placements/:id`; the Web
  workbench route is `/workspaces/:placement_id`.
- One Project may own Placements on multiple Nodes. Core creates Project and
  Placement identifiers; Node canonicalizes and validates paths and reports
  local facts.
- Heartbeat discovery and explicit binding converge on the same Placement for
  a Node/path pair. Discovery creates or refreshes one unbound Placement;
  explicit binding attaches it to a selected or newly created Project. A path
  alone never implies cross-node Project identity.

### Durable State Authority

Core owns durable product state, legal domain transitions and the global event
record. Numbered SQLx migrations with checksums define its schema. Unknown
persisted enum or state values are corruption or compatibility errors, not
fallbacks to an initial state. Every duplicated index or projection has one
documented authority and rebuild rule; normalized capability rows are
authoritative; immutable event envelopes and their searchable projections are
committed in one transaction; session/runtime links have one authoritative
relation. Enrollment claim, Project/Placement binding, session creation, turn
submission and event ingestion are units of work, and in-memory notification
occurs only after commit.

Node owns one transactional local state store and the long-lived local
resources it operates. A daemon-level `NodeSupervisor` owns registration,
heartbeat snapshots, command deduplication, runtime metadata, outbox state and
shutdown. One state-store actor is the only durable-state writer. The 0.2.0
store is SQLite, with separate tables for identity, command cache, outbox,
runtime metadata, transcripts and provider resume references. A
`RuntimeSupervisor` owns provider processes, cancellation, transcripts and
resume state; a `TerminalSupervisor` owns PTY children independently of a
control connection. Retention for completed commands, stopped runtimes,
transcripts and acknowledged outbox entries is explicit and bounded.

### Protocol V2 And Compatibility

Protocol v2 is one coordinated breaking release across Core, Node and Web.
Rust types in `uprava-protocol` are the source of truth; tracked JSON Schema,
TypeScript types, runtime validators and canonical fixtures are generated for
Web-facing roots and checked for drift. Built-in commands and events use tagged
typed payloads; known kinds do not use arbitrary JSON, while an explicit
extension variant remains available. Node-only control contracts do not enter
the browser bundle, and ingress validation proves scope, target and identifier
relationships.

Compatibility with 0.1.x APIs, schemas and state is not required. There is no
in-place 0.1.x migration: incompatible state must fail startup clearly with
reset and re-enrollment guidance, never be silently reinterpreted or deleted.
The first 0.2.0 run uses separate versioned Core and Node state/config slots;
the retained 0.1.8 Core database, Node JSON state and matching configuration
remain available for rollback. Rollback selects old binaries, configuration
and state together and does not carry work created only in 0.2.0 back to
0.1.8.

## Connection Model

The base safe model: Node Daemon establishes an outbound connection to Core.

This simplifies:

- NAT/firewall scenarios;
- connecting a personal computer;
- temporary devboxes;
- cloud nodes;
- mobile/web access.

Core then routes commands and streams through this connection.

Direct client-to-node connection can be considered later as an optimization for
local mode, but not as the required base architecture.

## Deployment Profiles

### Local Single-User

```text
same machine:
Core Backend + Web Control Panel + Node Daemon
```

Fits early MVP and local development.

### Personal Distributed

```text
server/VPS/cloud:
Core Backend + Web Control Panel

personal machines/devboxes:
Node Daemons

clients:
web/mobile/desktop/CLI
```

Fits the scenario "I work from computer and phone, while agents run on different
machines".

### Team/Cloud

```text
managed Core Backend
multiple users
multiple projects
multiple Node Daemons
shared workflows
role-based access
```

Fits commercial/team product.

## Open Questions

- Do we definitively call the entity `Node` rather than `Host`?
- Where should large artifacts be stored: Core storage, Node, or external object
  storage?
- Which secrets can live in Core, and which should stay only on Node?
- How should tool capabilities be described: MCP schema, custom contract, or
  adapter model?
- Where is the boundary between plugin, integration, tool, and visual block?
- Which integrations should use MCP, and which require a native adapter?
- How does tool/plugin versioning affect trace reproducibility?
- Should Core be able to execute lightweight tools itself, or should every
  execution go through Node/Provider?
- What minimum protocol is needed between Core and Node Daemon for MVP?
- Which transport should come first: HTTP polling, WebSocket, gRPC, message
  queue?
- How should terminal/filesystem commands be isolated in persistent session mode?
- How should the user see the difference between a Core-level tool and a
  Node-local tool?

## Current Position

At the current vision level, the strongest architectural position is:

- `Core Backend` is required as the control plane.
- `Web Control Panel` can be deployed together with Core immediately.
- `Node Daemon` is the system agent on a node and the main data plane.
- `AI Agents` are workloads, not infrastructure daemons.
- `Tool Registry` lives in Core.
- `Plugin Registry` lives in Core next to Tool Registry.
- External integrations connect through adapters: MCP, native API, Node-local,
  external provider, or hybrid.
- MCP matters as an integration protocol, but should not be the only extension
  mechanism.
- Tool execution can happen on Node, in an external provider, or later inside
  Core for safe lightweight tools.
- Clients should work through Core rather than directly owning distributed state.
