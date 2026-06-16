# Cortex Vision

Status: `draft`

## Short Formula

Cortex is a Distributed Agent OS for large-scale work with agents.

It should become a work operating environment where a human manages agents as distributed workloads: runs them on different nodes, sees their environment, controls progress, reviews output, accepts changes, and receives interactive visual artifacts instead of only text responses.

Current agent tools often look like chat interfaces with limited access to the environment. Cortex should be a control plane for agent work: projects, nodes, agents, files, terminals, diffs, tasks, workflows, plugins, artifacts, and decision trace.

## Product Thesis

AI agents are becoming capable enough to do long work, but the interfaces around them remain too flat. The user sees a chat and a final answer, but does not see enough of the workspace, changes, sources, checks, risks, intermediate decisions, and task state.

Cortex solves this not by building another chat, but by building an Agent OS:

- at the bottom: a Docker/Kubernetes-like model of nodes, node daemons, workspaces, and agent workloads;
- at the top: a Notion/Obsidian-like work surface with blocks, links, visualizations, and live artifacts;
- between them: a modular runtime that connects agents, tools, projects, task trackers, git, MCP, plugins, and review interfaces.

## Problem

Modern agent tools solve only part of the work:

- chat shows the dialogue, but does not give a full view into the project, filesystem, terminal, and environment state;
- agent output often becomes a text message like "I did X", a link, or a diff view without enough context;
- long work is hard to manage: it is difficult to assign a task for hours, leave, return, and quickly understand what happened;
- review and integration remain manual work, and the tool rarely reduces the cost of verification;
- mobile workflows are weak: reading chat is possible, but managing work, reviewing, inspecting files, checking state, and making decisions is hard;
- integrations exist as tools/connectors, but their results rarely become first-class UI;
- agent work is poorly traceable: the human is responsible for the result, but often cannot see enough trace for review, handoff, return, and rollback.

## Product Vision

Cortex should make agent work manageable.

The user does not merely write a prompt and wait. They choose a project, node, agent, execution mode, workflow, allowed scope, expected evidence, and acceptance criteria. Sometimes this is a live agent process the user can attach to and work with interactively. Sometimes it is a task-based run in an isolated environment. Sometimes it is a hybrid where a persistent session spawns bounded runs for specific subtasks.

The agent works in an observable environment. The system shows progress, changes, checks, risks, and artifacts. Output becomes accepted work only after review, correction, integration, and an ownership decision.

Core idea:

```text
agent output != accepted work

accepted work = output + validation + correction + integration + ownership decision
```

Cortex should make this cycle cheaper, clearer, and more comfortable.

## System Model

Base Cortex model:

- **Core / Control Plane** - central layer for projects, nodes, agents, tasks, workflows, artifacts, permissions, and state.
- **Node** - registered compute environment where work can run: local computer, server, devbox, microVM, cloud workspace, or sandbox.
- **Node Daemon** - system daemon on a node that registers the node in Core, launches agent runtimes through provider adapters, manages workspaces, and exposes files, terminal, processes, logs, and state.
- **Project** - work context: repository, documents, settings, agents, integrations, history, workflows, artifacts.
- **Workspace** - concrete execution environment: checkout, branch, sandbox, mounted files, env, running tools.
- **Agent Session** - live agent process or connection to an external agent that supports attach/detach, continued dialogue, state inspection, and environment control.
- **Agent Run** - bounded episode of agent work with goal, scope, context, events, logs, changes, checks, and result.
- **Agent Provider Adapter** - boundary that translates a concrete provider such as Codex, future OpenCode, or future Claude Code into Cortex launch, resume, stream, interrupt, stop, approval, and trace events.
- **Execution Mode** - how agent work runs: persistent session, task-based run, or hybrid mode.
- **Workflow** - durable work state that can survive agent, container, or node restarts.
- **Artifact** - output of agent work: text, diff, file, dashboard, UML, form, report, chart, embedded tool, or custom UI block.
- **Project Workspace Inspector** - non-chat workbench surface for a concrete workspace: file tree, file viewer, lightweight text editor, terminal/PTY sessions, command history, diffs, checks, and trace-linked workspace evidence.
- **Tool Registry** - registry of tools/capabilities in Core: metadata, schemas, permissions, routing, UI contracts, and audit policy.
- **Plugin** - extension that adds agents, tools, integrations, visual blocks, workflows, commands, or artifact types.

This model should start with developer workflows without becoming limited to them.

The client/server model is described in [architecture.md](architecture.md).

## Execution Modes

Cortex must not be locked into a single cloud-agent flow. Task-based sandboxing is important, but it is only one mode. The minimal model should support at least two modes, and later a hybrid between them.

### Persistent Agent Session

The agent runs as a live process or is connected as an external interactive agent. The user can attach to it, continue the dialogue, inspect terminal/logs/files, provide clarifications, and control the process almost like a work session.

Best for:

- exploratory work;
- collaborative design;
- tasks where context is clarified during work;
- local node work;
- cases where interactivity and continuity matter.

Key properties:

- attach/detach to a live agent;
- long-lived process context;
- visibility into files, terminal, commands, and current state;
- manual control over work;
- trace as a session journal and decision log, not only a final report.

### Task-Based Sandbox Run

The agent is called as a task executor. Core gives it a goal, tools, context package, sandbox/workspace, criteria, and expected evidence. The agent works in isolation and returns result, trace, changes, checks, and artifacts.

This resembles cloud agents and fits:

- bounded implementation tasks;
- background work for hours;
- CI/fix/review loops;
- reproducible workflows;
- tasks requiring isolation, branch, sandbox, or microVM;
- cases where the output is easier to review as a package of changes.

Key properties:

- bounded task input;
- sandbox/tool environment;
- event log;
- explicit stop condition;
- review-ready output;
- durable workflow state instead of attachment to a live process.

### Hybrid Managed Session

Hybrid mode connects an interactive persistent session and task-based sub-runs. The user works with a live agent or orchestration agent, and that agent can create isolated task runs for subtasks: test a hypothesis, make a diff, fix CI, prepare an artifact, or run a review.

This can feel close to modern cloud coding agents, but with stronger transparency: the user sees both the controlling session and the bounded runs it spawns.

Open design question: where is the boundary between live session context and reproducible state/trace of separate task runs?

## Principles

### 1. Distributed Agent OS, Not Chat App

Chat is important, but it is not the center of the system. The center is managed agent work: where it runs, what it can do, what files and tools it touched, what checks passed, what changed, and how the result can be accepted or rolled back.

### 2. Execution-Mode Neutral Core

Core should model persistent sessions and task-based runs as different modes of one system, not as two separate products. Projects, nodes, workspaces, tools, files, artifacts, trace, permissions, review, and integrations are shared. Lifecycle, isolation, state ownership, and review contracts differ.

### 3. Modularity as Architecture

Cortex should not try to replace Linear, GitHub, GitLab, Notion, Grafana, Docker, Temporal, sandbox providers, memory systems, or every MCP server. The stronger product position is to be a runtime, aggregator, and interface layer that connects these systems and makes them usable by both humans and agents in a shared work loop.

Plugins and integrations must be first-class architecture. Core stores the Tool Registry and Plugin Registry: which capabilities exist, where they came from, who can use them, where they execute, how they appear in UI, how they are traced, and which artifacts/workflows they add. MCP is an important integration path, but not the only one.

### 4. Visualization-First Output

Agent output should not be limited to text. If the result is better understood as a diff, table, chart, form, UML, dashboard, timeline, terminal replay, test report, dependency graph, or embedded external view, Cortex should show it as a first-class artifact.

Visualization is not decoration. It reduces the cost of understanding, review, and decision-making.

### 5. Traceability by Default

Meaningful agent work should leave a readable trace: goal, scope, context, constraints, key decisions, files/sources used, checks, results, unresolved risks, changed artifacts, next step, and reviewer decision.

Trace is not bureaucracy. It lowers review cost, return cost, handoff cost, and integration cost:

```text
trace -> lower review cost + lower return cost + better handoff + reusable memory
```

Trace must be proportional. A small task may need 2-4 lines. A large task needs a trace artifact, zone map, or review note. Too little trace makes the human reconstruct context again. Too much trace becomes unreadable log clutter.

### 6. Transparency and Right to Intervene

A human can be responsible for a result only if they have context, authority, resources, and ability to intervene. Cortex should show not only final output, but what was delegated, what the agent did, what was checked, what was not checked, where risks remain, and how to stop, fix, or roll back.

Practical test: a responsible person who did not participate in the agent dialogue should understand what was delegated, what was accepted, and what remains risky without asking the agent again.

### 7. Human-Agent Dual Interface

The interface should be usable by humans and accessible to agents. UI elements, artifacts, statuses, and actions should have machine-readable representations so an internal agent can understand what the user sees, help with navigation, explain state, and act with UI context.

### 8. Durable Workflows Over Long-Lived Containers

In task-based mode, the durable thing should be workflow state, not necessarily a specific process, container, or agent session. The agent can restart, workspace can be recreated, and node can change, while the system remembers where work stopped, which decisions were made, which checks are needed, and what the return trigger is.

In persistent mode, a long-lived process is a valid first-class execution mode, but it must still have observable state, attach/detach semantics, and trace.

### 9. Integration Over Reinvention

Cortex should first connect strong existing systems: git providers, task trackers, MCP, observability, sandbox runtimes, workflow engines, dashboards, memory tools. Own implementations are justified where a shared interface, coherence, UX, or traceability cannot be obtained through integration.

### 10. Mobile Continuity

Agent work should continue between computer and phone. Mobile should allow more than reading messages: understand task state, inspect trace, review diffs, make simple review decisions, stop an agent, answer blocking questions, and return work to progress.

### 11. Superadditive Work

Cortex should strengthen the human, not push them out of the loop. The goal is not maximum autonomy at any cost, but a human-agent-interface-trace system where speed, quality, understanding, skill, and safe delegation all improve.

## First Product Layer

The first version should lay the foundation for a Distributed Agent OS without trying to implement every direction.

Minimal foundation:

- Core with projects, nodes, agents, agent sessions, and agent runs;
- Node Daemon that can launch agents and report state;
- binding between agent session/run, project, workspace, and task;
- two architectural execution modes: persistent session and future task-based run;
- chat as one interface to session/run;
- Project Workspace Inspector for project/workspace file tree, file viewing, lightweight text editing, terminal/PTY sessions, command/output history, diffs, and check entry points;
- addressable workspace references connecting files, ranges, edits, terminal commands, diffs, checks, artifacts, and trace entries;
- basic trace for agent session/run;
- simple status model: draft / running / blocked / needs review / accepted / rejected / deferred;
- manual review loop: what changed, what was checked, what is risky, what next;
- minimal plugin/block contract for visual artifacts.

Base developer flows:

```text
persistent:
node/project -> start or attach agent session -> chat/files/terminal -> changes -> trace -> review

task-based:
task -> agent run -> sandbox/tools -> diff -> checks -> trace -> review -> MR/PR

hybrid:
agent session -> spawn bounded task runs -> review artifacts -> merge state back into session/workflow
```

The first layer should prove that Cortex gives more transparency and control than a regular agent chat.

## Product Development

The feature inventory is in [feature-inventory.md](feature-inventory.md).
Detailed product stages are in [product-stages.md](product-stages.md).

### Stage 1. Developer Node Workbench

First usable product for developer workflow:

- Core Backend and Web Control Panel;
- one or more nodes with Node Daemon;
- persistent Codex-backed session on a node through an Agent Provider Adapter;
- project/workspace binding;
- chat/session view;
- Project Workspace Inspector with file tree, file viewer, lightweight text editor, workspace terminal/PTY sessions, command/output history, and basic diff/check entry points;
- basic trace and event log;
- minimal Tool Registry, Plugin Registry, and visual block/artifact contract.

Task-based sandbox mode is not part of the first version. The architecture should leave room for it, but the first product focus is a persistent developer workbench.

### Stage 2. Modular Developer Workbench

Make modularity a real product capability:

- Tool Registry v1;
- Plugin Registry v1;
- integration adapter model;
- MCP adapter as one integration path;
- native adapters where MCP is not enough;
- first integrations: git provider and Linear;
- basic permissions and tool call trace;
- minimal visual plugin/block API.

### Stage 3. Visual Agent Work Surface

Make visualization a primary product difference:

- rich diff/review view;
- test/check report artifacts;
- trace timeline;
- terminal replay or structured command history;
- UML visualization/editor;
- dashboard artifacts;
- forms generated by agents/plugins;
- embedded external views.

### Stage 4. Task-Based Agent Runtime

Add the second execution mode:

- task-based sandbox runs;
- context package;
- isolated workspace/branch per run;
- expected evidence contract;
- durable workflow state;
- run queue;
- CI/webhook wakeups;
- review-ready output;
- MR/PR flow.

### Stage 5. Hybrid and Orchestrated Workflows

Connect persistent sessions and task-based runs:

- hybrid managed session;
- orchestration agent inside Cortex;
- session spawns bounded runs;
- semi-deterministic pipelines;
- skills/guides/guidelines library;
- review debt visibility.

### Stage 6. Multi-Node, Team and Cloud

Expand into a distributed/team platform:

- multi-user projects;
- roles and permissions;
- team audit trail;
- shared review queues;
- managed Core deployment;
- managed or registered cloud nodes;
- multi-node scheduling.

### Stage 7. Beyond Software Development

Apply the model to other kinds of work:

- research workflows;
- analytics and dashboards;
- documents and presentations;
- finance and monitoring;
- knowledge base workflows;
- personal/team agent processes.

## Success Metrics

Metrics should show quality of accepted work, not just speed of output generation:

- time from task definition to review-ready result;
- iterations to merge/acceptance;
- share of agent sessions/runs accepted without major human rewrite;
- review cost: time to understand, verify, and accept a result;
- unresolved risks at acceptance;
- returns to a task without context loss;
- average review debt;
- number of successful long tasks without constant human involvement;
- time to develop a new plugin/block/workflow;
- mobile completeness: how many decisions can be made from phone without desktop.

## Non-Goals

Early Cortex should not:

- build its own task tracker instead of Linear;
- build its own git provider instead of GitHub/GitLab;
- build a full memory system before validating runtime and workflow model;
- compete with Grafana/Notion/Obsidian as standalone products;
- build generic n8n-like automation before the agent model stabilizes;
- hide complexity of agent work behind a pretty "done" status.

## Open Questions

- What is the minimal work unit: task, agent run, workflow, or artifact?
- What is the top-level UX object: persistent session, task, workflow, or project work surface?
- How exactly should hybrid mode connect live sessions and isolated task runs?
- How strongly should the first product be tied to software development?
- Should durable workflow engine be built internally or integrated first?
- What minimal plugin/block API is needed in the first release?
- What does trace look like for small, medium, and large work?
- How do we separate useful traceability from log clutter?
- Where is the boundary between the internal Cortex agent and agents running on nodes?
- Which visual artifacts are needed first: diff, terminal, UML, dashboard, forms, test report?
- Which mobile scenario is first: monitoring, unblock, review, or task launch?
- What security boundaries are needed for daemon, files, terminal, and external integrations?

## Working Position

Strongest starting formulation:

Cortex is a control plane and work surface for agent workloads. It starts with software development because files, git, tests, diff, review, and MR/PR flow are clear there. But its base abstractions are broader than software development: node, node daemon, workspace, agent provider adapter, agent session, agent run, workflow, artifact, tool registry, plugin, and trace.

If this foundation is designed well, Cortex can grow into a modular operating system for human-agent work instead of another agent chat.
