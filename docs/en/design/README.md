# Uprava Design Phase

Status: `active`

This section exists for deep work on Uprava **key mechanisms**.

Here, design does not mean a list of internal modules, tables, and APIs. It
means hierarchical work on large product and architecture decisions:

1. First, formulate the vision of a key mechanism: the core idea, product logic,
   and why it matters for Uprava.
2. Then expand the vision into architecture: entities, boundaries, scenarios,
   lifecycle, contracts, artifacts/events, storage, permissions, UI
   consequences, and quality checks.

The same design doc starts with a root vision block and then gradually expands
into an architecture block. Inside architecture, depth can grow from a
conceptual model to technical contracts.

## What Is a Key Mechanism

A key mechanism is a large product-architecture mechanism in Uprava that defines
the work model for the user, agent, UI, and backend system.

It is not a separate code module, private feature, or user flow. A key mechanism
can contain many modules, scenarios, UI states, events, and technical decisions,
but the mechanism itself describes **how an important part of the system works**.

Examples of key mechanisms:

- distributed architecture: how Uprava implements the Core / Node Daemon /
  clients model;
- distributed runtime coordination: how Core coordinates runtime work on
  concrete Node/workspace placements, dispatch, events, stale/offline behavior,
  resource warnings, and overrides;
- modular UI: how the modular work surface, blocks, panels, and interface
  extensibility work;
- plugins and Tool Registry: how tools, plugins, integrations, MCP, native
  adapters, and visual blocks connect;
- dynamic UI: how an agent can return a form, dashboard, graph, embedded view,
  or another interactive block;
- visual rendering and artifact semantics: where and how Uprava renders visual
  objects, what is source-of-truth, and when a view becomes an artifact;
- go to source / causality navigation: how the user moves from a result, diff,
  error, or artifact to source, evidence, and cause;
- run mode: how Uprava runs agent work through Persistent Runtime,
  stateless/ephemeral runtime, or hybrid strategy, and how interactive session
  and bounded task contracts differ on top of that;
- human-agent dual interface: how humans and agents work with one visible model,
  where the agent is a first-class citizen.

Inside each key mechanism there can be many technical decisions, but the model
itself must be chosen first.

## How To Write Design Docs

Files in this directory should be named by key mechanism:

```text
docs/en/design/001-distributed-architecture.md
docs/en/design/002-run-mode.md
docs/en/design/003-distributed-runtime-coordination.md
docs/en/design/004-modular-ui-work-surface.md
docs/en/design/005-dynamic-ui-from-agents.md
docs/en/design/006-visual-rendering-and-artifact-semantics.md
docs/en/design/007-plugins-tool-registry-and-mcp-strategy.md
docs/en/design/008-go-to-source-and-causality-ux.md
docs/en/design/009-human-agent-dual-interface.md
docs/en/design/010-project-workspace-surface.md
```

Recommended document structure:

```text
# Key mechanism name

Status: draft / working-position / accepted / superseded

## Vision

### What problem the mechanism solves
### Conceptual implementation model
### User scenarios
### Agent-facing scenarios
### Scope boundaries / release constraints, if useful

## Architecture

### Core entities
### Responsibility boundaries
### UI consequences
### Detailed lifecycle and state machines
### API/protocol contracts
### Artifact/event formats
### Storage implications
### Permissions and failure modes
### Tests/evals/checklist
```

Main rule: first fix the **vision and essence of the mechanism**, then expand it
into **architecture**. Scenarios and UI already appear in `Vision`, but detailed
states, contracts, storage, and permissions live in `Architecture`.

## Cross-Cutting Principles

These principles should run through all key mechanisms:

- Uprava is a Distributed Agent OS, not agent chat with panels.
- Agent output is not accepted work.
- The product should reduce the cost of review, handoff, return, and ownership
  decisions.
- Modularity is an architectural principle, not a marketplace on top of a
  monolith.
- Visual artifacts and dynamic UI are ways to understand and control work, not a
  decorative layer.
- Integrations should be visible and traceable: not hidden API calls inside
  agent text.
- Persistent, stateless/ephemeral, and hybrid strategies should be runtime
  strategies of one Run Mode, not different products.
- Distributed Runtime Coordination should be a shared dispatch/resource-awareness
  layer for interactive sessions, future task runs, and sandboxed runtimes.
- Human UI and agent-readable UI should evolve together.
- Local development and UI verification are part of the system design: Docker
  Compose should provide a reproducible hardened Core/Web/Node setup, while
  Codex provider execution is covered by a separate real-provider smoke path.
  Playwright should cover both automated E2E checks and agent/operator
  inspection through `playwright-cli`.
- V01 should be small, but should not block plugins, visual blocks, task-based
  runtime, mobile, and team/cloud.

## Key Mechanism Map

| ID | Mechanism | Key questions | Expected result |
| --- | --- | --- | --- |
| A-001 | Distributed architecture | How exactly do we implement the distributed model? What is Core/control plane, what remains Node Daemon/data plane, how do clients work through Core, and where are the host/node/workspace/session boundaries? | Working position on the Core / Node Daemon / clients model, deployment profiles, and responsibility boundaries. |
| A-002 | Run Mode | What is Run Mode as one mechanism for starting agent work? How do Persistent Runtime, stateless/ephemeral runtime, and hybrid strategy work? How do interactive session and bounded task contracts differ on top of runtime strategy? Where are the boundaries between project, workspace, node, thread, turn, run, and agent process? | Run Mode concept for V01 and beyond: Persistent Runtime first, managed process lifetime, lifecycle, visible surface, review points, and constraints for future stateless/sandboxed strategies. |
| A-003 | Distributed Runtime Coordination | How does Core coordinate runtime work between session thread, runtime session, workspace placement, and Node? How are commands dispatched, events ordered, and node/workspace tree, stale/offline state, resource warning badges, and overrides shown in UI? How do git repo/branch signals show possible conflicts without a lock system? | Working coordination-layer model for V01: Nodes -> Projects/Workspaces tree, command proxy, idempotency, event ordering, resource signals, warning badges, override events, and reuse by future task/sandbox runtimes. |
| A-004 | Modular UI and work surface | What does modular UI mean for Uprava? Is it Notion-like blocks, IDE/workbench panels, Obsidian-like navigation, plugin-rendered surfaces, or a hybrid? Where are the boundaries between pages, panels, blocks, artifacts, integration surfaces, and extension points? | Work surface model: layout, blocks, panels, navigation, plugin surfaces, and constraints for React/Vite UI. |
| A-005 | Dynamic UI from agents | How should an agent return a form, dashboard, chart, graph, embedded tool, or custom block? Is this schema-driven UI, prebuilt block types, sandboxed components, generated code, or plugin-owned renderer? | Dynamic UI concept: what the agent can produce itself, what must be pre-registered, and where the safety boundary is. |
| A-006 | Visual rendering and artifact semantics | Where and how does Uprava render visual objects: inline Markdown diagrams, editor/viewer enhancements, diff/terminal/test views, charts, dashboards, external previews, and artifacts? What is source-of-truth, when does a visual view become an artifact, and which refs/actions/fallbacks are needed? | Cross-cutting visual object semantics model: source-of-truth, rendering scope, addressability, actions, fallback, ownership, cause refs, and artifact promotion. |
| **A-007** | Plugins, Tool Registry and MCP strategy | Where does Tool Registry live? Do we need a Core-level MCP gateway/proxy? Or should MCP live closer to Node Daemon, agent process, plugin adapter, or external provider? How do we compare MCP, native adapters, and hybrid adapters? | Tools/plugins/integrations model: registry, execution location, routing, permissions, events, and visual output. |
| A-008 | Go to source and causality UX | How do we make an equivalent of go to definition, but for agent work? How does a user go from answer, diff line, failed check, artifact, decision, status, or UI block to source/evidence/cause: prompt/context/tool call/command/event/file change/raw log? What is a source/cause graph, and what is log noise? | Causality UI/UX model: navigation from result to source, evidence, and cause, with a minimum source/cause link model that does not dump raw trace. |
| **A-009** | Human-agent dual interface and Agent as First-Class Citizen | How do we make UI understandable to both humans and agents? What are machine-readable UI state, context entry points, internal Uprava agent, chat over UI element, agent identity, capabilities, status, memory, permissions, and ownership? | Dual-interface model where the agent is a visible system participant, not a hidden process behind text chat. |
| A-010 | Project Workspace Surface | How does the user see and change a concrete agent workspace? Where do file tree, file viewer/editor, terminal/PTY, command history, diff/check views, and "open full IDE" sidecar live? How do Core/Node Daemon provide permissions, path boundaries, edit lifecycle, trace, and addressable workspace refs? | Post-V01 workspace surface model: inspect-first, edit-light, terminal-capable, traceable, with optional full IDE sidecar later. |

Not every important topic is a separate key mechanism. Some should stay as user
scenarios or slices inside design docs:

- Distributed Agent Control Panel is the main V01 scenario for `A-002 Run Mode`,
  `A-003 Distributed Runtime Coordination`, and `A-004 Modular UI and work
  surface`. Developer workbench surfaces start as post-V01 feature queue slices,
  beginning with workspace references and read-only inspector.
- Workflow and harness is a scenario slice for long work that validates `A-002 Run Mode` and `A-003 Distributed Runtime Coordination`, but does not replace them.
- Integration UX is a specific case of modularity, plugins/tools, and visual blocks; it should be expanded inside `A-004`, `A-005`, `A-006`, and `A-007`.
- Security, permissions, and trust are required architecture slices for execution
  modes, plugins/tools, dynamic UI, and agent identity. Security baseline is a
  first post-V01 implementation slice, but not a separate key mechanism on the
  design map.
- Metrics, observability, and evals are quality/feedback slices for checking mechanisms, not a separate key mechanism.
- Mobile continuity, deployment/bootstrap, and beyond software development are important constraints/product horizons, but not key mechanisms of the current design phase.

## Depth of Work

Each key mechanism first needs a `Vision`. This is not a summary, but the root
semantic block: what problem is being solved, what model is proposed, which
human and agent-facing scenarios are key, and which scope/release constraints
are already clear if they are useful for the direction.

`Architecture` can be filled gradually. Not every key mechanism must immediately
reach detailed state machines, API contracts, storage implications, or
tests/evals. At the current stage, the important thing is to create a design doc
for every mechanism in the map, fix `Vision` in each, and then deepen
`Architecture` where the decision is critical for V01 or blocks neighboring
mechanisms.

## What Counts as a Useful Result of This Phase

The design phase will be useful if, after it, we understand:

- each key mechanism in the map has a design doc with a root `Vision` and an
  `Architecture` scaffold;
- how the key Uprava mechanisms work: distributed architecture, run mode,
  distributed runtime coordination, modular UI, plugins/tools, dynamic UI, and
  visual rendering/artifact semantics;
- which decisions are required for V01 and which only constrain the
  architecture;
- which ideas from Notion/Obsidian/IDE/Grafana/MCP we adopt and which we do not;
- where the boundary is between product concept, architecture, and
  implementation detail;
- which documents need translation into `docs/en` when the position stabilizes.

## Top-Level Open Questions

- Should dynamic UI be part of the modular UI system or a separate
  artifact/runtime layer?
- Should we start with a set of fixed block types or immediately design
  plugin-rendered blocks?
- Should Core be an MCP gateway/proxy, or is it better to keep MCP closer to
  Node/agent/plugin execution?
- Where is the boundary between plugin, integration, tool, block, and artifact?
- How do we study Notion-like modularity practically: as data model, UI
  composition, plugin model, or interaction pattern?
- Where should visual representation be inline/viewer enhancement, a separate
  block, an artifact, or external preview/embed?
- What minimum source/cause graph is needed for go to source so it helps review
  without becoming a noisy trace log?
- How exactly should an agent be represented in UI as a first-class citizen:
  identity, status, permissions, memory, capabilities, or separate work object?
- How do we separate interactive session contract and bounded task contract
  inside Run Mode so they use a shared project/workspace/node/agent/artifact/event
  model?
- How do we avoid building an over-abstract platform before a working developer
  workbench exists?
