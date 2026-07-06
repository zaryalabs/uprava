# Uprava Vision

Status: `active`

## Short Formula

Uprava is a Distributed Agent OS for large-scale work with agents.

The product should become an operating work environment where a human manages
agents as distributed workloads: starts them on different nodes, sees their
environment, controls progress, reviews results, accepts changes, and receives
not only text answers but also interactive visual artifacts.

If current agent tools most often look like chat with limited access to the
environment, Uprava should be the control plane for agent work: projects, nodes,
agents, files, terminals, diffs, tasks, workflows, plugins, artifacts, and the
trail of decisions.

## Product Thesis

AI agents are becoming strong enough to perform long tasks, but the interfaces
around them remain too flat. The human sees chat and a final answer, but poorly
sees the work environment, changes, sources, checks, risks, intermediate
decisions, and task state.

Uprava solves this not through "one more chat", but through an Agent OS:

- below - a Docker/Kubernetes-like model of nodes, node daemons, work
  environments, and runnable agent workloads;
- above - a Notion/Obsidian-like work surface with blocks, links,
  visualizations, and live artifacts;
- between them - a modular runtime that connects agents, tools, projects, task
  trackers, git, MCP, plugins, and review interfaces.

## Problem

Modern agent tools cover only part of the work:

- chat shows dialogue, but does not provide a full view of the project, file
  system, terminal, and environment state;
- agent results are often reduced to text such as "I did X", a link, or a diff
  view without rich context;
- long tasks are hard to manage: it is difficult to assign a task for hours,
  leave, return, and quickly understand what happened;
- review and integration remain manual overhead, while the tool does little to
  reduce verification cost;
- the mobile scenario is weak: a user can read chat, but cannot properly manage
  work, review, inspect files, check status, and make decisions;
- integrations exist as tools/connectors, but their results rarely become
  first-class UI;
- agent work is poorly traced: the human remains responsible for the result, but
  often lacks enough trail for review, handoff, return, and rollback.

## Product Vision

Uprava should become a system where agent work has a manageable shape.

A human does not only write a prompt and wait for an answer. They choose a
project, node, agent, execution mode, workflow, allowed boundaries, expected
evidence, and acceptance criteria. Sometimes this is a live agent process that
the human can attach to and work with interactively. Sometimes this is a
task-based run in an isolated environment. Sometimes this is a hybrid where a
persistent session spawns bounded runs for individual subtasks.

The agent works in an observable environment. The system shows progress,
changes, checks, risks, and artifacts. The result becomes an accepted work item
only after review, correction, integration, and an ownership decision.

Core idea:

```text
agent output != accepted work

accepted work = output + validation + correction + integration + ownership decision
```

Uprava should make this cycle cheaper, clearer, and easier to operate.

## System Model

Base Uprava model:

- **Core / Control Plane** - the central layer for managing projects, nodes,
  agents, tasks, workflows, artifacts, permissions, and state.
- **Node** - a registered compute environment where work can run: local
  computer, server, devbox, microVM, cloud workspace, or sandbox.
- **Node Daemon** - a system daemon on the node that registers the node in Core,
  starts agents, manages workspaces, and provides access to files, terminal,
  processes, logs, and state.
- **Project** - the work context: repository, documents, settings, agents,
  integrations, history, workflows, and artifacts.
- **Workspace** - the concrete execution environment for a task: checkout,
  branch, sandbox, mounted files, environment, and running tools.
- **Agent Session** - a live agent process or connection to an external agent
  that can be attached/detached, continued through dialogue, inspected, and
  controlled.
- **Agent Run** - a bounded work episode with a goal, scope, context, events,
  logs, changes, checks, and result.
- **Execution Mode** - the way agent work runs: persistent session, task-based
  run, or hybrid mode.
- **Workflow** - durable work state that can survive an agent, container, or node
  restart.
- **Artifact** - an output of agent work that can be text, a diff, a file,
  dashboard, UML, form, report, chart, embedded tool, or custom UI block.
- **Tool Registry** - the Core registry of tools/capabilities: metadata,
  schemas, permissions, routing, UI contracts, and audit policy.
- **Plugin** - an extension that adds agents, tools, integrations, visual
  blocks, workflows, commands, or new artifact types.

This model should let Uprava start with developer workflows without being
trapped inside them.

The client/server model is described in more detail in
[architecture.md](architecture.md).

## Execution Modes

Uprava should not be tied to one cloud-agent flow. The task-based sandbox
approach matters, but it is only one mode. The minimum model should support at
least two modes, and later a hybrid between them.

### Persistent Agent Session

An agent starts as a live process or connects as an external interactive agent.
The user can attach to it, continue dialogue, inspect terminal/logs/files, give
clarifications, and control the process almost like a work session.

This mode fits:

- exploratory work;
- collaborative design;
- tasks where context is refined during work;
- work with a local node;
- cases where interactivity and continuity matter.

Key properties:

- attach/detach to a live agent;
- long-lived process context;
- visibility into files, terminal, commands, and current state;
- manual control over work progress;
- trace as a session log and record of important decisions, not only a final
  report.

### Task-Based Sandbox Run

The agent is invoked as a task executor. Core gives it a goal, tools, context
package, sandbox/workspace, criteria, and expected evidence. The agent works in
isolation and returns a result, trace, changes, checks, and artifacts.

This mode is similar to cloud agents and fits:

- bounded implementation tasks;
- background work for hours;
- CI/fix/review loops;
- reproducible workflows;
- tasks that need isolation, branch, sandbox, or microVM;
- cases where it is easier to review the result as a package of changes.

Key properties:

- bounded task input;
- sandbox/tool environment;
- event log;
- explicit stop condition;
- review-ready output;
- durable workflow state instead of being tied to a live process.

### Hybrid Managed Session

Hybrid mode connects a persistent interactive session and task-based subruns.
The user works with a live agent or orchestration agent, and that agent can
create isolated task runs for individual subtasks: check a hypothesis, make a
diff, run a CI fix, prepare an artifact, or perform review.

This mode can feel close to current cloud coding agents, but with more
transparency: the user sees both the controlling session and the separate
bounded runs it creates.

The key design question is where the boundary sits between live session context
and reproducible state/trace of individual task runs.

## Principles

### 1. Distributed Agent OS, Not Chat App

Chat is an important interface, but not the center of the system. The center is
manageable agent work: where it runs, what it is allowed to do, which files and
tools it touched, which checks passed, what changed, and how the result can be
accepted or rolled back.

### 2. Execution-Mode Neutral Core

Core should model agent work so that persistent sessions and task-based runs are
different modes of one system, not two different products. Shared concepts
should include projects, nodes, workspaces, tools, files, artifacts, trace,
permissions, review, and integrations. Lifecycle, isolation, state ownership,
and review contract should differ.

### 3. Modularity as Architecture

Uprava should not try to immediately replace Linear, GitHub, GitLab, Notion,
Grafana, Docker, Temporal, sandbox providers, memory systems, and all MCP
servers. The strong product position is to be a runtime, aggregator, and
interface layer that connects these systems and makes them accessible to humans
and agents through one work loop.

Plugins and integrations should be first-class parts of the architecture. Core
stores the Tool Registry and Plugin Registry: which capabilities are available,
where they came from, who can use them, where they execute, how they appear in
UI, how they are traced, and which artifacts/workflows they add. MCP is an
important integration path, but not the only one.

### 4. Visualization-First Output

Agent output should not be limited to text. If a result is better understood as
a diff, table, chart, form, UML, dashboard, timeline, terminal replay, test
report, dependency graph, or embedded external view, Uprava should be able to
show it as a first-class artifact.

Visualization is not a decorative layer. It reduces the cost of understanding,
review, and decision-making.

### 5. Traceability by Default

Meaningful agent tasks should leave a readable trail: goal, scope, context,
constraints, key decisions, used files/sources, checks, results, unresolved
risks, changed artifacts, next step, and reviewer decision.

Trace is not bureaucracy. It reduces the cost of review, returning to a task,
handoff, and integration:

```text
trace -> lower review cost + lower return cost + better handoff + reusable memory
```

Proportionality matters: a small task can leave 2-4 lines, while a large task
needs a separate trace artifact, zone map, or review note. Too little trace
forces a human to reconstruct context. Too much trace will not be read.

### 6. Transparency and Right to Intervene

A human can be responsible for a result only if they have context, authority,
resources, and the ability to intervene. Uprava should show not only final
output, but also what was delegated, what the agent did, what was checked, what
was not checked, where risks are, and how to stop, fix, or roll back an action.

Practical test: a human who did not participate in the agent dialogue should be
able to understand what was delegated, what was accepted, and what remains
risky, without asking the agent again.

### 7. Human-Agent Dual Interface

The interface should be convenient for humans and accessible to agents. UI
elements, artifacts, statuses, and actions should have machine-readable
representations so an internal agent can understand what the user sees, help
with navigation, explain state, and act with UI context.

### 8. Durable Workflows Over Long-Lived Containers

In task-based mode, workflow state should be durable, not necessarily a specific
process, container, or agent session. An agent can be restarted, a workspace can
be recreated, and a node can change, but the system should remember where work
stopped, which decisions were made, which checks are needed, and what is the
return trigger.

In persistent mode, a long-lived process is valid as a first-class execution
mode, but it still needs observable state, attach/detach semantics, and trace.

### 9. Integration Over Reinvention

Uprava should first connect strong existing elements: git providers, task
trackers, MCP, observability, sandbox runtimes, workflow engines, dashboards,
and memory tools. Uprava-owned implementations are needed where a shared
interface, coherence, UX, or traceability cannot be achieved through
integration.

### 10. Mobile Continuity

Work with agents should continue between desktop and phone. The mobile scenario
should allow a user not only to read messages, but also to understand task
state, inspect trace, review diffs, make simple review decisions, stop an agent,
answer blocking questions, and return a task to work.

### 11. Superadditive Work

Uprava should amplify the human, not remove them from the process. The goal is
not maximum autonomy at any cost, but a combination of human, agents, interface,
and decision trail where speed, quality, understanding, skill, and safe
delegation all improve.

## First Product Layer

The first version should lay the foundation of a Distributed Agent OS without
trying to implement every direction at once.

Minimum foundation:

- Core with projects, nodes, agent sessions, runtimes, messages, and events;
- Node Daemon on a node that can start a Codex-backed runtime and report state;
- binding an agent session to a node, project, and workspace;
- one implemented execution mode: persistent interactive session;
- task-based and hybrid modes preserved as architecture directions, not V01
  implementation;
- chat as the first interface to a session;
- `Nodes -> Projects/Workspaces -> Sessions` navigation;
- lifecycle controls: start, attach, detach, interrupt, stop, resume, and return
  later where provider support allows it;
- basic status model for node, project/workspace, runtime, and session;
- basic event history and diagnostics for lifecycle, offline, stale, warning, and
  error states;
- trusted local/single-user or controlled development deployment, with security
  baseline as the first hardening slice after V01;
- UI shell and entity model prepared for later file browser, terminal, diff,
  trace, tools, plugins, review, and visual artifact surfaces.

Base developer flows:

```text
persistent:
node/project/session tree -> start or attach agent session -> chat -> lifecycle/events -> stop/resume

task-based:
future task -> agent run -> sandbox/tools -> diff -> checks -> trace -> review -> MR/PR

hybrid:
future session -> spawn bounded task runs -> review artifacts -> merge state back into session/workflow
```

The goal of the first layer is to prove that Uprava gives more transparency and
control than a regular chat with an agent.

## Product Development

The canonical first product version is described in [v01.md](v01.md). The queue
of next implementation slices is described in
[feature-queue.md](feature-queue.md). The map of possible product evolution is
described in [product-evolution.md](product-evolution.md). A detailed inventory
of already proposed features and directions lives in
[feature-inventory.md](feature-inventory.md).

The first product version is **V01 Distributed Agent Control Panel**:

- Core Backend and Web Control Panel;
- one or more nodes with Node Daemon;
- persistent Codex-backed session through Agent Provider Adapter;
- `Nodes -> Projects/Workspaces -> Sessions` navigation tree;
- project/workspace binding as placement context;
- chat/session view as the first primary work surface;
- session lifecycle controls: start, attach, detach, interrupt, stop, resume,
  and return later where provider support allows it;
- basic node, project, runtime, session, message, and event persistence;
- UI shell and typed command/event envelopes shaped for future workspace,
  editor, terminal, tools, plugins, trace, and artifact surfaces.

After V01, development is better handled as a feature queue: each key mechanism
can have a small useful slice and then grow toward the target shape.

## Success Metrics

Metrics should measure not only output generation speed, but also the quality of
accepted work:

- time from task creation to review-ready result;
- number of iterations until merge / acceptance;
- share of agent runs accepted without major manual rewrite;
- review cost: how much time it takes to understand, verify, and accept a
  result;
- number of unresolved risks at acceptance time;
- frequency of returning to a task without losing context;
- average size of review debt;
- number of successful long tasks without constant human participation;
- time to develop a new plugin/block/workflow;
- mobile completeness: how many decisions can be made from a phone without
  switching to desktop.

## Non-Goals

In early stages, Uprava should not:

- build its own task tracker instead of Linear;
- build its own git provider instead of GitHub/GitLab;
- build a full memory system before validating runtime and workflow models;
- compete with Grafana/Notion/Obsidian as standalone products;
- build universal automation like n8n before a stable agent model exists;
- hide the complexity of agent work behind a nice "done" status.

## Open Questions

- What is the minimum unit of work: task, agent run, workflow, or artifact?
- What object is top-level in the UX: persistent session, task, workflow, or
  project work surface?
- How exactly should hybrid mode between live session and isolated task runs
  work?
- How tightly should the first product be tied to software development?
- Should we build a durable workflow engine as our own layer or integrate an
  existing one first?
- What minimum plugin/block API is needed in the first version?
- What does a trace artifact look like for a small, medium, and large task?
- How do we separate useful traceability from log noise?
- Where is the boundary between the internal Uprava agent and agents that run on
  nodes?
- Which visual artifacts are needed in the first release: diff, terminal, UML,
  dashboard, forms, test report?
- Which mobile scenario should come first: monitoring, unblock, review, or task
  launch?
- What security constraints are needed for daemon, files, terminal, and external
  integrations?

## Working Position

The strongest initial formulation:

Uprava is a control plane and work surface for agent workloads. It starts with
software development because files, git, tests, diff, review, and MR/PR flow are
clear there. But its base abstractions should be broader than development: node,
node daemon, workspace, agent session, agent run, workflow, artifact, tool
registry, plugin, and trace.

If this foundation is built correctly, Uprava can grow not into another agent
chat, but into a modular operating system for human-agent collaboration.
