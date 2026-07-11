# Uprava

Uprava is a Distributed Agent OS for large-scale work with AI agents.

The product starts with software development workflows and can expand into analytics, research, finance, documents, and broader knowledge work as the base model transfers. The first practical direction is a developer-focused workbench for live agent work running on nodes through a managed-lifetime Persistent Runtime run mode and a distributed runtime coordination layer.

## Product Thesis

Most agent tools are still shaped like chat. They can run an agent and show the result, but they do not provide enough visibility into files, terminal, workspace state, changes, checks, trace, visual artifacts, or review flow.

Uprava should be a control plane and work surface for agent workloads:

- **Core Backend** as the control plane.
- **Node Daemon** as the data plane running on local machines, servers, devboxes, sandboxes, or cloud nodes.
- **Web Control Panel** as the first client.
- **Project Workspace Inspector** as the non-chat workbench surface for browsing project files, viewing and lightly editing text files, attaching workspace terminals, inspecting diffs/checks, and linking evidence back to trace.
- **Run Mode and Agent Provider Adapter** as the execution abstractions, starting with managed-lifetime Persistent Runtime for live agent work and Codex as the first provider.
- **Distributed Runtime Coordination** as the dispatch, event ordering, node/workspace placement, and resource-warning layer between Core, Node Daemon, and runtime processes.
- **Tool Registry and Plugin Registry** as the foundation for modularity.
- **Visual artifacts and traceability** as first-class product principles.

## Current State

The current repository state is the unique `0.2.0-rc.6` candidate built on the
working `0.1.8` shipped baseline, plus
product and architecture documentation. `V01` names the first product cut that
shipped as `0.1.0`; the current implementation has moved through five completed
feature queue slices, one unified audit hardening slice, and one workspace
renderer/PTY terminal slice plus the first self-hosted CI/CD deploy baseline
after that cut.

Synchronized product and architecture docs:

- [Vision](docs/en/vision.md)
- [Architecture](docs/en/architecture.md)
- [V01](docs/en/v01.md)
- [Versioning](docs/en/versioning.md)
- [Releases](docs/en/releases.md)
- [Feature Queue](docs/en/feature-queue.md)
- [Product Evolution](docs/en/product-evolution.md)
- [Product Stages](docs/en/product-stages.md) - superseded historical stage model
- [Tech Stack](docs/en/tech-stack.md)
- [Feature Inventory](docs/en/feature-inventory.md)
- [Project Workspace Inspector](docs/en/workspace-inspector.md)
- [Workspace Editing and IDE Sidecar](docs/en/workspace-editing-and-ide-sidecar.md)
- [Self-Hosting Golden Path](docs/en/self-hosting-golden-path.md)
- [Unified Architecture and Code Quality Audit](docs/en/audit/audit-2026-07-09.md)
- [0.2.0 Portable Polish Handoff](docs/polish/README.md)
- [Design Docs](docs/en/design)
- [Source Notes](docs/en/uprava-notes.md)
- [TMP Plans](docs/tmp-plans) - temporary implementation plans for intermediate development slices

Russian mirror and source notes:

- [Russian docs](docs/ru)

## First Product Version

V01 is **Distributed Agent Control Panel**:

- Rust Core Backend;
- Rust Node Daemon;
- web control panel;
- managed-lifetime Persistent Runtime as the first run mode, with Codex through a provider adapter;
- distributed runtime coordination with a `Nodes -> Projects/Workspaces -> Sessions` tree, command dispatch, event ordering, and resource/offline warning badges;
- project/workspace binding as placement context;
- chat/session view as the first primary work surface;
- persistent session lifecycle: start, attach, detach, interrupt, stop, resume, and return later where provider support allows it;
- basic node, project, runtime, session, message, and event persistence;
- UI shell and typed command/event envelopes shaped so workspace inspector, editor, terminal, tools, plugins, trace, and artifact surfaces can be added later without reshaping the product model.

For the first Codex adapter, V01 treats the persistent runtime as a
Core-managed session with persisted state, ordered events and provider resume
references. Codex continuity uses the stable `codex exec` and
`codex exec resume` paths where a provider session id is available; a
provider-native live process owner, live output streaming and real Codex
interrupt escalation are follow-on work.

V01 should feel like a small control panel for a distributed agent system: only
slightly more visible than chat at first, but already organized around nodes,
projects, sessions, runtime state, and durable event history. Project Workspace
Inspector, tools, plugins, dynamic UI, and visual artifacts move to the feature
queue instead of being required in the first cut.

V01 is a trusted local/single-user or controlled development deployment, not a
production security release. Security baseline is the first post-V01 hardening
slice.

The detailed first-version scope lives in [V01](docs/en/v01.md). Follow-on work is tracked as an implementation queue in [Feature Queue](docs/en/feature-queue.md), not as a fixed phase-based roadmap.

`V01` remains the historical first product cut. Current implementation version
and shipped post-`0.1.0` slices are tracked in
[`Versioning`](docs/en/versioning.md) and [`Releases`](docs/en/releases.md).

## Preliminary Tech Stack

```text
Rust workspace
Axum Core Backend
Rust Node Daemon
SQLite
HTTP + WebSocket/SSE
Docker Compose local development profile
React 19 + TypeScript + Vite
Tailwind CSS v4
shadcn/ui conventions
lucide-react
TanStack Query
TanStack Table
React Hook Form + Zod
Vitest
Playwright UI testing and agent verification
Rust tooling: cargo, rust-analyzer, rustfmt, clippy, bacon, nextest, audit, deny, taplo
```

Next.js is not the required V01 runtime. It remains an option for cloud/web frontend, BFF, SSR, public pages, or SaaS needs if those become strong enough reasons.

Local development should have a Docker Compose dev profile that starts the
stable Core/Web path with predictable ports, Core state volume, healthcheck and
reset behavior. Run the Node Daemon as a host process when it needs real local
workspace and provider access.
UI verification should use Playwright in two modes: automated E2E tests and
agent/operator inspection through `playwright-cli` against the same local setup.

## Local Development Scaffold

The `0.1.8` implementation baseline now includes:

- Rust workspace crates for shared protocol/domain contracts, Core Backend and
  Node Daemon;
- Vite React Web Control Panel under `apps/web`;
- SQLite-backed Core skeleton with health, inventory, heartbeat, placement,
  session, Codex provider, artifact tree and agent projection APIs;
- Project Workspace Inspector with safe file reads, text saves, bounded
  workspace command execution, command history and diff/check entry points;
- Monaco-backed file/diff rendering and xterm.js-backed interactive workspace
  PTY terminal sessions routed through Core and owned by Node;
- unified audit hardening for quality gates, Node state/file safety, command
  retry semantics, session stream cursors, healthcheck and web error states;
- GitHub Actions release automation, deploy manifests and server activation
  scripts for the self-hosted Core/Web/Node release path;
- Docker Compose dev profile for Core and Web, plus host Node Daemon run and
  smoke paths.

Start the local stack from separate terminals:

```sh
make init
make core-r
make node-r
make web-r
```

`make node-r` defaults the Node workspace allow-list to this repository root.
Set `UPRAVA_NODE_WORKSPACES=/path/to/workspace-root` before running it when the
Node should manage a different local workspace tree.

Or use the Docker Compose dev profile for Core/Web:

```sh
make dev-up
```

The current local runbook is
[`docs/en/runbooks/v01-local-dev.md`](docs/en/runbooks/v01-local-dev.md).

## Documentation Workflow

Documentation is split by language and should stay path-synchronized:

- [`docs/en`](docs/en) - English-facing documentation mirror.
- [`docs/ru`](docs/ru) - Russian documentation, drafts, source notes, and design work.
- [`docs/tmp-plans`](docs/tmp-plans) - temporary implementation plans for active intermediate development slices.

Synchronization rules:

1. Keep the same relative Markdown document set in `docs/en` and `docs/ru`.
2. If one language has a document that the other language lacks, add the missing mirror instead of deleting the source document.
3. If both language versions exist but the product or architecture content conflicts, the Russian version has priority and the English version should be updated to match it.
4. Deep design documents may start in Russian and be mirrored first so the document set stays complete; translate or polish the English-facing text incrementally without removing the Russian source position.

TMP Plans are intentionally tactical and are not part of the `docs/en` and
`docs/ru` mirror set. If a temporary plan creates a durable product,
architecture, or process decision, promote that decision into the synchronized
canonical documentation.

## Background

Uprava is built around practices from harness engineering and Superadditivity Theory: the goal is not maximum AI autonomy at any cost, but a human-agent system where speed, quality, understanding, traceability, review capacity, and safe delegation improve together.
