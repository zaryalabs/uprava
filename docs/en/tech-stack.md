# Cortex Tech Stack

Status: `draft`

This document records the preliminary technical stack for Stage 1 and nearby stages.

## Short Decision

Stage 1 is a Rust-first system with web-first UI:

```text
Rust Core Backend + Rust Node Daemon
React 19 + TypeScript + Vite SPA
Tailwind CSS v4
shadcn/ui conventions
lucide-react
TanStack Query
TanStack Table
React Hook Form + Zod
Vitest
Rust tooling: cargo, rust-analyzer, rustfmt, clippy, bacon, nextest, audit, deny, taplo
```

Next.js 16 App Router is not the Stage 1 base. Not because Next.js is bad, but because Cortex already has a Rust Core Backend. In the first stage we should avoid adding a second Node.js backend/BFF layer without a strong reason.

## Architecture Position

Core Backend and Node Daemon should be written in Rust.

Web UI should be a frontend app that talks to Core API and event streams. In Stage 1, it can be served by Rust Core as static assets.

Tauri is considered a desktop shell/client, not the product core.

```text
Core Backend        Rust / Axum / Tokio
Node Daemon         Rust / Tokio
CLI                 Rust
Web Control Panel   React / TypeScript / Vite
Desktop Client      Tauri later, wraps Web Control Panel or talks to Core
```

## Rust Stack

### Core Backend

Preliminary choice:

- Rust;
- Tokio;
- Axum;
- Serde;
- SQLx or SeaORM later, decision deferred;
- SQLite for local/single-user first;
- Postgres-compatible architecture later;
- WebSocket or SSE for live session/events;
- structured logging/tracing.

Core Backend owns:

- API for clients;
- Web Control Panel delivery;
- auth/session model;
- Node registry and discovery;
- project registry;
- agent sessions/runs registry;
- event log;
- trace metadata;
- artifact metadata;
- Tool Registry;
- Plugin Registry;
- integration configuration;
- routing to Node Daemon.

### Node Daemon

Preliminary choice:

- Rust;
- Tokio;
- outbound connection to Core;
- local workspace management;
- PTY/terminal support;
- process lifecycle management;
- agent provider adapter lifecycle;
- file operations;
- persistent agent sessions;
- Codex provider adapter as the first implementation;
- normalized agent runtime events;
- output/event streaming;
- local tool execution.

Node Daemon should be portable. Stage 1 targets desktop/server nodes, but the architecture should not block future cloud nodes, sandboxes, and microVM hosts.

### Agent Provider Adapters

Stage 1 can be Codex-first, but the launch boundary should be provider-shaped.

Provider adapters translate concrete agent behavior into Cortex runtime
contracts:

- capability discovery;
- runtime start/resume in a workspace;
- user turn or task input submission;
- provider output streaming;
- approval/user-input request mapping;
- interrupt and stop;
- provider session id / resume cursor extraction;
- normalized lifecycle and trace events.

Codex-specific protocol code should live in the Codex adapter or Node-local
runtime layer. Core-facing types should use provider-neutral concepts:
`provider_id`, `runtime_session_id`, `session_thread_id`, `turn_id`,
`runtime_strategy`, `work_contract`, `status`, `event`, `approval`, `trace`,
`artifact`, and opaque `provider_resume_ref` where needed.

Future adapters for OpenCode, Claude Code, or other agents should be able to
reuse the same minimal contract. They do not need to reach feature parity with
Codex in Stage 1.

### CLI

CLI should also be written in Rust so it can reuse shared crates and API client.

Possible CLI tasks:

- start local Core;
- register Node;
- inspect nodes/sessions;
- connect to Core;
- run diagnostics;
- manage plugins/tools later.

### Rust Tooling

Baseline Rust tooling:

- `cargo` - build/test/package tool.
- `rust-analyzer` - required language server.
- `rustfmt` - consistent formatting.
- `cargo clippy` - linting and correctness checks.
- `bacon` - local watcher for fast dev loop.
- `cargo-nextest` - primary test runner for workspace tests.
- `cargo audit` - known vulnerability checks for dependency tree.
- `cargo deny` - licenses, advisories, duplicate dependencies, and dependency policy.
- `taplo-cli` - format/check TOML files.

Preliminary local dev loop:

```text
cargo fmt
cargo clippy --workspace --all-targets
cargo nextest run --workspace
```

For daily development, use `bacon` to continuously run `check`, `clippy`, or targeted tests while editing.

Preliminary CI/security baseline:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo audit
cargo deny check
taplo fmt --check
taplo lint
```

`cargo audit` and `cargo deny` do not replace each other. `audit` covers known security advisories. `deny` covers broader dependency policy: licenses, bans, advisories, duplicated crates, and sources.

## Frontend Stack

### Base

Stage 1 choice:

- React 19;
- TypeScript;
- Vite;
- Tailwind CSS v4;
- shadcn/ui conventions;
- lucide-react icons;
- TanStack Query;
- TanStack Table;
- React Hook Form;
- Zod;
- Vitest.

### Why Vite SPA, Not Next.js in Stage 1

Vite SPA is simpler for the current architecture:

- Core Backend already exists in Rust;
- web app can be served as static assets from Core;
- simpler self-hosting and local single-user deployment;
- easier to wrap with Tauri later;
- less risk of blurring responsibility between Rust Core and Node.js BFF;
- realtime UI will revolve around Core API, WebSocket/SSE, and client state anyway.

Next.js can return later if one of these appears:

- separate cloud/web frontend deployment;
- BFF layer for web is needed;
- Server Components/Server Actions provide a strong benefit;
- public/marketing/docs pages require SSR/SEO;
- multi-tenant SaaS frontend makes Next.js clearly useful.

### UI Conventions

Use shadcn/ui as a convention and component source model, not as an external black-box component library.

This fits Cortex:

- components live in project code;
- they can be adapted to the product;
- component APIs are predictable for humans and AI agents;
- it is suitable for building a custom design system;
- custom workbench components can be layered over base primitives.

Use lucide-react as the default icon set.

### Project Workspace Inspector UI

Stage 1 Web Control Panel must support the Project Workspace Inspector:

- stable workbench layout with project tree, file/detail area, terminal panel,
  and diff/check entry points;
- file viewer/editor with line/range navigation, lightweight text editing,
  dirty state, explicit save/apply, and safe fallbacks for large, binary,
  missing, ignored, or permission-denied files;
- terminal/PTY panel backed by Core-routed Node Daemon streams;
- command/output history linked to session events and trace;
- addressable workspace references for files, ranges, edits, terminal commands,
  diffs, checks, and artifacts.

The exact terminal and code editor component libraries are deferred. Likely
directions are CodeMirror or Monaco for file viewing/editing and diff surfaces,
and xterm.js for terminal rendering. Keep them behind local component boundaries
such as `FileViewer`, `FileEditor`, `DiffViewer`, and `TerminalPanel` so the
first implementation can stay small and be replaced if richer editing, replay,
or a full IDE sidecar is needed later.

### State and Data Fetching

TanStack Query is the default for server state:

- nodes;
- sessions;
- projects;
- workspaces;
- tools;
- plugins;
- artifacts;
- traces;
- event snapshots;
- review state.

Realtime updates in Stage 1 can be built as:

```text
HTTP queries for snapshots
WebSocket/SSE for events
TanStack Query cache updates from event stream
```

Do not add a global client state manager until there is a clear need. Local component state + URL state + TanStack Query should be enough.

### Tables

TanStack Table is used for:

- nodes list;
- sessions list;
- tools/plugins registry;
- artifacts;
- events;
- future review queues;
- future task runs.

### Forms and Validation

React Hook Form + Zod are used for:

- project settings;
- node setup;
- tool/plugin configuration;
- integration credentials forms;
- session launch forms;
- future task run forms.

Zod is useful as the frontend validation boundary. Backend contracts should remain Rust-first; generated schemas can be considered later.

### Testing

Stage 1:

- Vitest for unit/component logic;
- Rust tests for core/node crates.

Later:

- Playwright for Web Control Panel e2e;
- integration tests for Core <-> Node Daemon protocol;
- scenario/eval tests for agent workflows.

## Tauri

Tauri v2 is not the Stage 1 foundation, but remains a strong candidate for desktop client.

Possible Tauri roles:

- desktop shell around Web Control Panel;
- local launcher for Core + Node Daemon;
- tray app;
- local notifications;
- OS integration;
- easier local credentials handling;
- desktop-specific UX.

Rule: shared domain logic must not live inside `src-tauri`. Keep it in Rust crates so Core, Node Daemon, CLI, and Tauri can reuse the same code.

## Repository Shape

Preliminary structure:

```text
crates/
  cortex-core/        shared domain model and contracts
  cortex-agents/      agent provider and runtime contracts
  cortex-server/      Core Backend
  cortex-node/        Node Daemon and provider adapter host
  cortex-client/      Rust API client
  cortex-tools/       tool/plugin contracts
  cortex-events/      event and trace contracts

apps/
  web/                React + Vite Web Control Panel
  cli/                Rust CLI
  desktop/            Tauri client later
```

This is not final, but it reflects the main split:

- Rust crates own system contracts and runtime;
- web app owns UI;
- desktop app is optional shell;
- Core and Node remain separate deployable binaries.

## Deferred Decisions

- SQLx vs SeaORM vs another DB layer.
- SQLite-only first or immediate SQLite/Postgres abstraction.
- WebSocket vs SSE for event streams.
- OpenAPI vs custom generated client vs shared schema generation.
- Whether frontend lives under `apps/web` with Vite or later moves to Next.js.
- Whether Tauri appears in Stage 1 as launcher or waits until Stage 2/3.
- Exact package manager for frontend.
- Exact monorepo tooling for frontend.

## Current Recommendation

Initial stack:

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

Stage 1 should avoid:

- Next.js as required app runtime;
- Node.js BFF layer;
- full workflow engine;
- full plugin marketplace;
- Tauri-specific domain logic.
