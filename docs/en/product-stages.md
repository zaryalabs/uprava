# Cortex Product Stages

Status: `draft`

This document describes product development stages. The key idea: each stage should produce a qualitatively new product, not just a longer feature list.

## Stage Principles

- Each stage should be usable end-to-end.
- Each stage should validate one main product thesis.
- First-stage architecture must leave room for registry, plugins, visual blocks, integrations, and task-based mode.
- First product focus is developer workflow.
- First execution mode is persistent agent session on nodes.
- Codex is the first agent provider implementation, not the long-term product boundary.
- Stage 1 should include a minimal Agent Provider Adapter boundary even if the first adapter is Codex-optimized.
- Task-based sandbox mode is important, but not needed in the first version.
- A small coherent working system is better than a full cloud-agent runtime built too early.

## Stage 1. Developer Node Workbench

**Qualitatively new product:** Cortex can be used as a web control panel for working with a live development agent on one or more nodes.

It is not just chat. The user sees project, agent session, terminal, files, changes, basic trace, and node state.

Stage 1 technical base is described in [tech-stack.md](tech-stack.md): Rust Core/Node, React 19 + TypeScript + Vite, Tailwind CSS v4, shadcn/ui conventions, lucide-react, TanStack Query/Table, React Hook Form + Zod, and Vitest.

### Main Thesis

Persistent agent session + Node Daemon + Core Backend already provide more control, transparency, and mobility than a regular local agent chat.

### Scope

- Core Backend as control plane.
- Web Control Panel as first client.
- Node Daemon as data plane.
- Node registration in Core.
- Basic node discovery and heartbeat.
- Project registry.
- Project/workspace binding on node.
- Persistent agent session mode.
- Minimal Agent Provider Adapter contract.
- Codex provider adapter as default AI agent runtime.
- Attach/detach to agent session.
- Chat/session view.
- Terminal/output view.
- File browser.
- Basic diff view.
- Basic session trace.
- Basic event log.
- Minimal status model for session/workspace/node.
- Minimal Tool Registry shape in Core.
- Minimal Plugin Registry shape in Core.
- Minimal visual block/artifact contract.

### Out of Scope

- Task-based sandbox runs.
- Durable workflow engine.
- Multi-agent orchestration.
- Full plugin marketplace.
- Complex RBAC/team model.
- Managed cloud nodes.
- Production support for additional providers such as OpenCode or Claude Code.
- Full provider-neutral feature parity across CLI agents.

### Why This

Persistent mode is simpler and closer to the first developer use case: run an agent on your node, attach to it, inspect its environment, and control work. It validates the base Cortex value faster: control plane + node daemon + transparent UI.

The first implementation may overfit to Codex where that is pragmatic, but the
launch/resume/control boundary should still be expressed as an Agent Provider
Adapter. Core and UI should talk about providers, runtimes, sessions, turns,
events, approvals, files, diffs, and trace, not Codex-specific process details.

Task-based mode needs more infrastructure: sandbox lifecycle, workflow state, queues, review contracts, artifact packaging, retries, webhook wakeups. It should come after the base model of nodes, sessions, files, terminal, diff, and trace works.

### Readiness Criterion

The user can:

1. Start Core and Web Control Panel.
2. Connect local or remote Node.
3. Open a project on Node.
4. Start persistent Codex-backed session through the Agent Provider Adapter.
5. Inspect chat, terminal output, files, diff, and trace.
6. Stop/continue work and return to the session later.

## Stage 2. Modular Developer Workbench

**Qualitatively new product:** Cortex becomes an extensible platform for developer workflows, not a hardcoded workbench.

Registry contracts and first integrations become real.

### Main Thesis

Cortex modularity must be a system capability, not a future promise.

### Scope

- Tool Registry v1.
- Plugin Registry v1.
- Integration adapter model.
- First-class MCP adapter support.
- Native adapter path for integrations where MCP is not enough.
- First extension path for additional agent provider adapters when a concrete provider is selected.
- Git provider integration.
- Linear integration as first task tracker.
- Basic plugin configuration UI.
- Basic permissions for tools.
- Tool call trace.
- Artifact metadata tied to tool/plugin output.
- Minimal visual plugin/block API.
- CLI access to registered tools.

### Product Shift

Stage 1 is a workbench for live agent sessions.

Stage 2 is a workbench that can be extended: tools, integrations, visual blocks, and workflows start connecting through a clear model.

## Stage 3. Visual Agent Work Surface

**Qualitatively new product:** Cortex becomes a visual work surface for agent work, not a set of panels around chat.

### Main Thesis

Cortex's advantage over agent chat is not only control plane, but visual representation of result, state, and review.

### Scope

- Rich diff/review view.
- Test/check report artifacts.
- Trace timeline.
- Terminal replay or structured command history.
- UML visualization.
- Basic UML editor or diagram artifact editing.
- Dashboard artifact support.
- Forms generated by agents/plugins.
- Embedded external views, e.g. Grafana.
- Artifact gallery per project/session.
- Visual block layout in chat/session pages.

### Product Shift

Stage 2 connects tools and plugins.

Stage 3 turns their results into first-class UI artifacts that can be inspected, reviewed, and used without reading long agent text.

## Stage 4. Task-Based Agent Runtime

**Qualitatively new product:** Cortex becomes a runtime for background agent tasks, not only interactive sessions.

This stage adds the second execution mode: task-based sandbox run.

### Main Thesis

After Core, Node Daemon, registry, trace, and visual artifacts exist, Cortex can safely add a cloud-agent-like task flow.

### Scope

- Task-based agent run mode.
- Context package.
- Sandbox/workspace lifecycle.
- Isolated branch or workspace per run.
- Explicit stop condition.
- Expected evidence contract.
- Run event log.
- Review-ready output.
- Durable workflow state.
- Basic run queue.
- Git webhook wakeups.
- CI follow-up loop.
- MR/PR flow.
- Review queue.

### Product Shift

Stage 3: user works with live agents and visual artifacts.

Stage 4: user can put bounded tasks in the background and receive review-ready output.

## Stage 5. Hybrid and Orchestrated Workflows

**Qualitatively new product:** Cortex connects interactive and background modes.

### Main Thesis

A strong agent workflow does not have to be only live session or only task run. A live session can spawn bounded runs, and a task-based workflow can return to interactive review.

### Scope

- Hybrid managed session.
- Orchestration agent inside Cortex.
- Session spawns task runs.
- Task run results merge back into session/workflow state.
- Multi-step workflow templates.
- Semi-deterministic pipelines: implement -> self-check -> review -> fix.
- Skills/guides/guidelines library.
- Agent self-review and check steps.
- WIP limits and review debt visibility.

### Product Shift

Stage 4 adds background tasks.

Stage 5 turns them into managed agentic workflows.

## Stage 6. Multi-Node, Team and Cloud

**Qualitatively new product:** Cortex becomes a distributed/team platform.

### Main Thesis

After validating single-user and personal distributed scenarios, Cortex can expand into a team/cloud model.

### Scope

- Multi-user projects.
- Roles and permissions.
- Team audit trail.
- Shared review queues.
- Managed Core deployment.
- Managed or registered cloud nodes.
- Node pools.
- Multi-node scheduling.
- Organization-level plugin/integration management.
- Stronger secrets model.
- Billing/account/project model if commercial path is chosen.

### Product Shift

Stage 5 is an advanced personal/small-team Agent OS.

Stage 6 is a managed distributed Agent OS for teams.

## Stage 7. Beyond Software Development

**Qualitatively new product:** Cortex becomes a general WorkOS for agents.

### Main Thesis

Developer workflow is the first vertical. The base model is broader: nodes, agents, tools, visual artifacts, trace, and workflows apply to research, analytics, documents, finance, and knowledge work.

### Scope

- Research workflows.
- Analytics workflows.
- Document and presentation workflows.
- Finance/monitoring workflows.
- Knowledge base workflows.
- Domain-specific visual artifacts.
- Domain-specific plugins and templates.

### Product Shift

Stage 6 makes Cortex a mature distributed platform.

Stage 7 expands it beyond software development.

## Preliminary Sequence

```text
1. Developer Node Workbench
   persistent sessions, node daemon, core, web control panel

2. Modular Developer Workbench
   tool registry, plugin registry, integrations

3. Visual Agent Work Surface
   first-class artifacts, visual review, dashboards/forms/UML

4. Task-Based Agent Runtime
   sandboxed runs, durable workflow state, MR/PR flow

5. Hybrid and Orchestrated Workflows
   sessions spawn runs, pipelines, skills/guides

6. Multi-Node, Team and Cloud
   collaboration, RBAC, managed core/nodes

7. Beyond Software Development
   research, analytics, docs, finance, knowledge workflows
```

## Open Questions

- Should Stage 1 include basic git integration, or is diff view over workspace enough?
- Does Stage 1 need mobile UI, or is responsive web enough for monitoring/attach?
- What is the minimal trace form for persistent session?
- Should Tool Registry in Stage 1 be only an internal schema, or have UI?
- Where is the boundary between Stage 2 visual block API and Stage 3 rich visual artifacts?
- When does MR/PR flow appear: Stage 2 as git integration or Stage 4 as task-based output?
- Should Stage 4 use an existing workflow engine or a minimal internal durable state first?
