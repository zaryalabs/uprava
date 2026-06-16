# Cortex Architecture

Status: `draft`

This document records the first architectural position on the client/server model for Cortex.

## Short Decision

Cortex should have a separate **Core Backend** as the control plane. Clients work through Core, while concrete work with machines, files, terminals, processes, sandboxes, and AI-agent lifecycle happens through **Node Daemons**.

Core is a required architectural abstraction, but not necessarily a remote SaaS backend in every deployment. In local mode, Core can run on the same machine as the UI and Node Daemon.

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
files, terminal, processes, sandboxes, agent runtime lifecycle, provider adapters
        |
        v
Agent Provider Runtimes / Tools / Workspaces
Codex first, future providers, persistent sessions, task runs, hybrid flows
```

## Terms

### Core Backend

Central backend and control plane for Cortex.

Core owns the global system model: projects, users, permissions, nodes,
capabilities, agent providers, agent sessions, agent runs, workflows, artifacts,
event log, trace, tool registry, routing, and web control panel.

Core should not become a process that directly works with every machine's filesystem. Concrete environment work should remain on the Node Daemon side.

### Control Panel

Web UI deployed with Core Backend and used to manage Cortex from the browser.

At the early stage this can be the primary client. Desktop, mobile, and CLI clients can come later.

### Client

User interface to Core.

Client types:

- web;
- desktop;
- mobile;
- CLI.

A client should not be required to connect directly to every node. Base model: client talks to Core, Core routes commands, events, and state between client and nodes.

### Node

Registered compute environment where work can run.

Node can be:

- local computer;
- server;
- devbox;
- cloud workspace;
- sandbox;
- microVM host;
- future managed cloud node.

`Host` can be used as a technical explanation, but the product entity is better named `Node`, because Cortex is closer to a distributed/cloud model than to a simple list of machines.

### Node Daemon

System daemon running on a Node.

This is not an AI agent. Node Daemon is infrastructure. It:

- registers Node in Core;
- reports capabilities;
- starts and stops AI-agent runtimes through provider adapters;
- manages persistent agent sessions;
- runs task-based work in sandbox/workspace;
- exposes files;
- opens terminal/PTY or command execution;
- streams logs, events, and outputs;
- applies changes;
- runs checks/tests;
- manages local workspaces, env, credentials, and runtime limits.

Node Daemon is the main Cortex data plane.

### AI Agent

AI-agent workload launched through Node Daemon or connected as an external provider.

AI Agent can work in different execution modes:

- persistent agent session;
- task-based sandbox run;
- hybrid managed session.

### Agent Provider

Concrete agent implementation that can perform AI work.

Codex is the first provider Cortex should support in Stage 1. Future provider
adapters should be possible for OpenCode, Claude Code, and other coding or
domain agents without changing the Core product model.

Provider-specific behavior includes:

- launch command or protocol;
- session or conversation identity;
- resume mechanism;
- output/event format;
- approval and user-input requests;
- interrupt/stop semantics;
- tool and filesystem permission model;
- provider-specific local state.

Provider-specific details should stay at the adapter edge unless they are
normalized into Cortex concepts such as session, turn, runtime status, event,
approval, artifact, trace entry, diff, or check result.

### Agent Runtime

Live or recoverable execution instance of an Agent Provider.

Depending on provider and execution mode, an Agent Runtime can be:

- a long-lived CLI or local process;
- a CLI process resumed by provider-native session id;
- a connection to an external provider service;
- a sandboxed process inside a task workspace;
- a future managed runtime slot.

An Agent Runtime is not the durable product object. The durable objects are
session thread, workspace binding, run/workflow state, event log, artifacts,
trace metadata, and provider resume reference when available.

### Agent Provider Adapter

Boundary owned by Node Daemon or an external provider integration that translates
provider-specific behavior into Cortex lifecycle commands and events.

The adapter should provide a minimal contract:

```text
discover capabilities
start runtime in workspace
resume runtime from provider resume reference when possible
submit user turn or task input
stream provider output as normalized Cortex events
map provider approval/user-input requests to Cortex requests
interrupt runtime
stop runtime
report runtime status and exit reason
extract provider session id / resume cursor when available
```

Stage 1 can have one production adapter for Codex and may still be optimized
around Codex behavior. The architectural rule is that Core, UI, trace, and
workflow state should not be named or shaped as if Codex is the only possible
agent. Codex-specific protocol fields can exist in adapter-local state or in an
opaque provider reference, not as the general Cortex runtime contract.

The first provider-neutral surface should stay intentionally small. Cortex does
not need universal feature parity across every CLI agent before it has one good
Codex-backed persistent runtime.

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

Without Core, every client would need to discover nodes, hold connections, understand capabilities, and synchronize state. This breaks quickly in mobile, distributed, and team scenarios.

### Mobile and Web Access

Phone or browser should not connect directly to a laptop, devbox, or sandbox. They need a stable endpoint.

Core provides that endpoint and allows the user to:

- open web UI;
- inspect task state;
- attach to an agent session;
- read trace;
- inspect diff/artifacts;
- make review decisions;
- stop or continue work.

### Workflow State

Task-based mode, hybrid mode, CI callbacks, and long-running work require durable state:

- what was started;
- where work stopped;
- which checks passed;
- which webhook arrived;
- what the next step is;
- who needs to decide.

This state should live in Core, not in a specific client.

### Trace and Event Log

Traceability should be shared across all clients and nodes. Core stores event log, trace metadata, artifact metadata, review decisions, and workflow state.

Node Daemon can store local raw logs or large files, but Core should know what exists, where it lives, who can access it, and how it connects to workflow.

### Security and Permissions

Core is where system-level decisions are made and checked:

- who sees a project;
- who sees a Node;
- who can open terminal;
- who can start an agent run;
- who can use a tool;
- who can read an artifact;
- who can accept a diff;
- who can stop or delete a session.

Node Daemon must enforce local restrictions, but policy and routing should be coordinated through Core.

## Tool Registry

Tool Registry should live in Core.

Tools are part of shared capabilities, permissions, UI, trace, and routing. If the registry only lives on nodes or clients, Core cannot answer:

- which tools are available in a project;
- which tools are available on a specific Node;
- which tools a user or agent can use;
- which tools can appear in UI;
- how a tool maps to a visual block or artifact;
- which tool calls must be traced;
- where to route a tool call;
- which schemas, permissions, and risk levels a tool has.

Tool execution does not have to happen in Core.

Model:

```text
Core Tool Registry
metadata, schema, permissions, routing, UI contract, audit policy

Node Tool Runtime
local execution, files, terminal, local env, local credentials

External Tool Provider
MCP, SaaS API, GitHub, Linear, Grafana, Docker, MLflow, etc.
```

Core knows that a tool exists and how to work with it. Node Daemon or external provider executes the action where data, credentials, and runtime live.

## Plugins and Integrations

Plugins and integrations are one of the main modularity mechanisms in Cortex.

Cortex should not implement every external system itself. Core should provide an extensible model for connecting:

- task trackers: Linear, Jira, GitHub Issues;
- knowledge and docs systems: Notion, Obsidian-like repos, Google Docs;
- git providers: GitHub, GitLab;
- observability and dashboards: Grafana, LangSmith, Langfuse, OpenTelemetry, Phoenix;
- runtimes and infrastructure: Docker, sandbox providers, devboxes, Kubernetes-like environments;
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
- permissions requested by plugin;
- integration accounts/connections;
- compatibility with Core and Node Daemon versions.

Tool Registry owns concrete callable capabilities. Plugin Registry owns package-level extension: where a tool came from, which UI/artifact/workflow extensions it added, how it is configured, and how it updates.

### Integration Adapters

An integration can connect in different ways:

- **MCP adapter** - when the external system is already available through MCP or MCP is a good tool protocol.
- **Native API adapter** - when we need control over auth, pagination, webhooks, rate limits, domain objects, or visual UX.
- **Node-local adapter** - when the tool must run near files, terminal, local credentials, or runtime.
- **External provider adapter** - when the tool executes in an external SaaS/provider.
- **Hybrid adapter** - metadata and permissions live in Core, execution happens through Node or external provider.

MCP is important, but it should not be the only integration mechanism. Cortex needs more than tool calls:

- show tools in UI;
- trace calls;
- connect output to artifact/workflow;
- apply permissions;
- support review;
- embed visualizations;
- make results understandable to both human and agent.

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

This makes integrations first-class parts of the system instead of hidden API calls behind an agent text response.

## Responsibility Split

### Core Backend Owns

- API for clients;
- web control panel;
- auth and user/session management;
- projects;
- Node registry and discovery;
- Node capabilities;
- agent session/run registry;
- agent provider capability metadata;
- workflow state;
- event log;
- trace metadata;
- artifact metadata;
- Tool Registry;
- Plugin Registry;
- integration registry and configuration;
- permissions and policies;
- command routing to Node Daemons;
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
- agent provider adapter lifecycle;
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
- following mode-specific contract.

## Connection Model

Base secure model: Node Daemon establishes an outbound connection to Core.

This simplifies:

- NAT/firewall scenarios;
- connecting a personal computer;
- temporary devboxes;
- cloud nodes;
- mobile/web access.

Core then routes commands and streams through that connection.

Direct client-to-node connection can be considered later as a local-mode optimization, but not as the required base architecture.

## Deployment Profiles

### Local Single-User

```text
same machine:
Core Backend + Web Control Panel + Node Daemon
```

Good for early MVP and local development.

### Personal Distributed

```text
server/VPS/cloud:
Core Backend + Web Control Panel

personal machines/devboxes:
Node Daemons

clients:
web/mobile/desktop/CLI
```

Good for "work from computer and phone, agents run on different machines".

### Team/Cloud

```text
managed Core Backend
multiple users
multiple projects
multiple Node Daemons
shared workflows
role-based access
```

Good for a commercial/team product.

## Open Questions

- Do we finalize `Node` instead of `Host`?
- Where do large artifacts live: Core storage, Node, or external object storage?
- Which secrets can live in Core, and which must remain only on Node?
- How should tool capabilities be described: MCP schema, own contract, or adapter model?
- Where is the boundary between plugin, integration, tool, and visual block?
- Which integrations should use MCP, and which need native adapters?
- How does versioning tools/plugins affect trace reproducibility?
- Should Core execute lightweight tools itself, or should every execution go through Node/Provider?
- What is the minimal Agent Provider Adapter contract needed for the Codex-first MVP?
- Which provider-specific resume/session fields can Core persist, and which must stay Node-local or opaque?
- When should OpenCode and Claude Code adapters become product requirements rather than compatibility tests?
- What minimal Core <-> Node Daemon protocol is needed for MVP?
- Which transport comes first: HTTP polling, WebSocket, gRPC, message queue?
- How do we isolate terminal/filesystem commands in persistent session mode?
- How do we show the difference between Core-level tools and Node-local tools?

## Current Position

Current strongest architecture position:

- `Core Backend` is required as the control plane.
- `Web Control Panel` can be deployed with Core.
- `Node Daemon` is the system daemon on a node and the main data plane.
- `AI Agents` are workloads, not infrastructure daemons.
- `Agent Provider Adapter` is the boundary between Cortex runtime contracts and provider-specific launch/resume protocols.
- Codex is the first provider implementation, but Core concepts should remain provider-neutral.
- `Tool Registry` lives in Core.
- `Plugin Registry` lives in Core next to Tool Registry.
- External integrations connect through adapters: MCP, native API, Node-local, external provider, or hybrid.
- MCP is important, but should not be the only extension mechanism.
- Tool execution can happen on Node, in an external provider, or later in Core for safe lightweight tools.
- Clients should work through Core and not directly own distributed state.
