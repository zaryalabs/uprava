# Cortex

Cortex is a Distributed Agent OS for large-scale work with AI agents.

The product starts with software development workflows and can later expand into analytics, research, finance, documents, and broader knowledge work. The first practical direction is a developer-focused workbench for live agent work running on nodes through a managed-lifetime Persistent Runtime run mode.

## Product Thesis

Most agent tools are still shaped like chat. They can run an agent and show the result, but they do not provide enough visibility into files, terminal, workspace state, changes, checks, trace, visual artifacts, or review flow.

Cortex should be a control plane and work surface for agent workloads:

- **Core Backend** as the control plane.
- **Node Daemon** as the data plane running on local machines, servers, devboxes, sandboxes, or cloud nodes.
- **Web Control Panel** as the first client.
- **Run Mode and Agent Provider Adapter** as the execution abstractions, starting with managed-lifetime Persistent Runtime for live agent work and Codex as the first provider.
- **Tool Registry and Plugin Registry** as the foundation for modularity.
- **Visual artifacts and traceability** as first-class product principles.

## Current Stage

The current repository stage is product and architecture design.

Canonical English docs:

- [Vision](docs/en/vision.md)
- [Architecture](docs/en/architecture.md)
- [Product Stages](docs/en/product-stages.md)
- [Tech Stack](docs/en/tech-stack.md)
- [Feature Inventory](docs/en/feature-inventory.md)

Russian drafts and source notes:

- [Russian docs](docs/ru)

## First Product Direction

Stage 1 is **Developer Node Workbench**:

- Rust Core Backend;
- Rust Node Daemon;
- web control panel;
- managed-lifetime Persistent Runtime as the first run mode, with Codex through a provider adapter;
- project/workspace binding;
- chat/session view;
- terminal/output view;
- file browser;
- basic diff view;
- basic trace and event log;
- minimal Tool Registry, Plugin Registry, and visual block/artifact contract.

Stateless/sandboxed run strategies, durable workflow engine, and full MR/PR flow are intentionally deferred.

## Preliminary Tech Stack

```text
Rust workspace
Axum Core Backend
Rust Node Daemon
SQLite
HTTP + WebSocket/SSE
React 19 + TypeScript + Vite
Tailwind CSS v4
shadcn/ui conventions
lucide-react
TanStack Query
TanStack Table
React Hook Form + Zod
Vitest
Rust tooling: cargo, rust-analyzer, rustfmt, clippy, bacon, nextest, audit, deny, taplo
```

Next.js is not the required Stage 1 runtime. It remains a deferred option for future cloud/web frontend, BFF, SSR, public pages, or SaaS needs.

## Documentation Workflow

The repository's primary language is English.

Documentation is split by language:

- [`docs/en`](docs/en) - canonical English documentation. After a document is translated here, this version becomes the source of truth.
- [`docs/ru`](docs/ru) - Russian drafts, notes, and intermediate documents. Use this folder when it is easier to shape product or architecture ideas in Russian first.

Default flow:

1. Draft and discuss in Russian under `docs/ru` when needed.
2. Stabilize the idea and translate it to `docs/en`.
3. Continue further work in English in `docs/en`.
4. Keep the Russian version as a draft/archive unless a new Russian discussion starts.

## Background

Cortex is built around practices from harness engineering and Superadditivity Theory: the goal is not maximum AI autonomy at any cost, but a human-agent system where speed, quality, understanding, traceability, review capacity, and safe delegation improve together.
