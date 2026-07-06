# Uprava Tech Stack

Status: `draft`

This document records the preliminary technical stack for V01 and the nearest
feature-queue slices.

## Short Decision

V01 should be built as a Rust-first system with a web-first UI:

```text
Rust Core Backend + Rust Node Daemon
Docker Compose local development profile
React 19 + TypeScript + Vite SPA
Tailwind CSS v4
shadcn/ui conventions
lucide-react
TanStack Query
TanStack Table
React Hook Form + Zod
Vitest
Playwright
Rust tooling: cargo, rust-analyzer, rustfmt, clippy, bacon, nextest, audit, deny, taplo
```

Next.js 16 App Router is not the V01 baseline for now. The reason is not that
Next.js is bad, but that Uprava already has a Rust Core Backend. In the first
product version, we should avoid creating a second backend/BFF layer on Node.js
without a strong reason.

## Architecture Position

Core Backend and Node Daemon should be written in Rust.

Web UI should be a regular frontend application that talks to Core API and event
streams. In V01, Rust Core can serve it as static assets.

Tauri is considered a desktop shell/client, but not the product core.

```text
Core Backend        Rust / Axum / Tokio
Node Daemon         Rust / Tokio
CLI                 Rust
Web Control Panel   React / TypeScript / Vite
Desktop Client      Tauri later, wraps Web Control Panel or talks to Core
```

## Local Development Environment

Docker Compose is the canonical local bootstrap and smoke-test environment for
V01 development. It is not a production deployment model; it is a stability tool
for making Core, Web, Node-facing protocol paths, hardened enrollment/auth and
diagnostics reproducible on every machine.

The baseline Compose setup should provide:

- predictable ports for Core and Web;
- persistent but resettable SQLite/Core state volumes;
- a hardened Core/Web/Node smoke path that can run without Codex;
- an option to run a Node Daemon in Compose for synthetic workspaces;
- an option to run Node Daemon on the host when it must touch real local
  workspaces and host credentials;
- health checks that are useful to `make`, Playwright and CI;
- documented reset and log-collection commands.

Real local workspace control can require a host-running Node Daemon. Compose
should still remain the stable way to start Core/Web/Node and the infrastructure
smoke path, while `make codex-smoke` covers real provider execution where Codex
is installed.

## Rust Stack

The current implementation baseline uses Rust `1.88` as the minimum supported
Rust version. This follows the locked dependency graph rather than pinning
transitive dependencies back to the older provisional toolchain.

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
- controlled-development deployment profile for V01;
- local auth/session, CSRF and Node enrollment/credential lifecycle basics;
- Node registry and discovery;
- project registry;
- agent sessions/runs registry;
- event log;
- trace metadata in later slices;
- artifact metadata in later slices;
- Tool Registry in later slices;
- Plugin Registry in later slices;
- integration configuration in later slices;
- routing to Node Daemon.

### Node Daemon

Preliminary choice:

- Rust;
- Tokio;
- outbound connection to Core;
- local workspace management;
- process lifecycle management;
- persistent agent sessions;
- output/event streaming;
- workspace binding validation for V01;
- PTY/terminal support in later workspace slices;
- file operations in later workspace slices;
- local tool execution in later Tool Registry slices.

Node Daemon should be maximally portable. V01 targets desktop/server nodes, but
the architecture should not block future cloud nodes, sandboxes, and microVM
hosts.

### CLI

The CLI is also better written in Rust, so it can reuse shared crates and the
API client.

Possible CLI tasks:

- start local Core;
- register Node;
- inspect nodes/sessions;
- connect to Core;
- run diagnostics;
- manage plugins/tools later.

### Rust Tooling

Base Rust tooling:

- `cargo` - main build/test/package tool.
- `rust-analyzer` - required language server for development.
- `rustfmt` - unified code formatting.
- `cargo clippy` - linting and correctness checks.
- `bacon` - local watcher for fast dev loop.
- `cargo-nextest` - main test runner for workspace tests.
- `cargo audit` - check known vulnerabilities in the dependency tree.
- `cargo deny` - licenses, advisories, duplicate dependencies, and dependency
  policy.
- `taplo-cli` - format/check TOML files.

Preliminary local dev loop:

```text
cargo fmt
cargo clippy --workspace --all-targets
cargo nextest run --workspace
```

For daily development, `bacon` can continuously run `check`, `clippy`, or
targeted tests while code changes.

Preliminary CI/dependency hygiene baseline:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo audit
cargo deny check
taplo fmt --check
```

`cargo audit` and `cargo deny` do not replace each other. `audit` handles known
security advisories; `deny` handles broader dependency policy: licenses, bans,
advisories, duplicated crates, and sources.

The repository keeps the dependency policy in `deny.toml` and TOML formatting
policy in `taplo.toml`. `make init` installs `cargo-audit`, `cargo-deny` and
`taplo-cli` if they are missing; `make c` requires them through `rust-dl`.
Taplo is currently used for formatting checks only.

## Frontend Stack

### Base

V01 choice:

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

### Why Vite SPA, Not Next.js In V01

Vite SPA is simpler for the current architecture:

- Core Backend already exists in Rust;
- the web app can be served as static assets from Core;
- self-hosting and local single-user deployment are simpler;
- wrapping it in Tauri later is simpler;
- there is less risk of blurring responsibility between Rust Core and a Node.js
  BFF;
- realtime UI will be built around Core API, WebSocket/SSE, and client state
  anyway.

Next.js can return later if one of these factors appears:

- a separate cloud/web frontend deployment;
- a BFF layer for web is needed;
- Server Components/Server Actions become a strong advantage;
- public/marketing/docs pages with SSR/SEO needs appear;
- a multi-tenant SaaS frontend appears where Next.js gives real value.

### UI Conventions

shadcn/ui is used as a convention and component source model, not as an external
black-box component library.

This fits Uprava well:

- components live in the project code;
- they can be adapted for the product;
- component APIs are predictable for humans and AI agents;
- it is convenient to build a custom design system;
- custom workbench components can be added on top of base primitives.

lucide-react is the default icon set.

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
- events snapshots;
- review state.

Realtime updates in V01 can be built as:

```text
HTTP queries for snapshots
WebSocket/SSE for events
TanStack Query cache updates from event stream
```

A global client state manager should not be added until there is a clear need.
Local component state + URL state + TanStack Query should be enough.

### Tables

TanStack Table is needed for:

- nodes list;
- sessions list;
- events;
- future tools/plugins registry;
- future artifacts;
- future review queues;
- future task runs.

### Forms and Validation

React Hook Form + Zod are needed for:

- project settings;
- node setup;
- session launch forms;
- future tool/plugin configuration;
- future integration credentials forms;
- future task run forms.

Zod is useful as a frontend validation boundary. Backend contracts should still
be Rust-first; generated schemas can be considered later.

### Testing

V01:

- Vitest for unit/component logic;
- Rust tests for core/node crates.
- Playwright for automated Web Control Panel E2E tests against the Docker
  Compose local profile;
- Playwright CLI for agent/operator UI verification while implementing: inspect
  the running app, click through flows, capture screenshots, and confirm that
  state is visible outside assistant text.

Later:

- integration tests for Core <-> Node Daemon protocol;
- scenario/eval tests for agent workflows.

The Playwright CLI mode is not a replacement for deterministic E2E coverage. It
is the interactive verification path agents use before handoff when a UI change
needs visual or workflow confirmation.

## Tauri

Tauri v2 is not the foundation for V01, but remains a strong candidate for a
desktop client.

Possible Tauri roles:

- desktop shell around Web Control Panel;
- local launcher for Core + Node Daemon;
- tray app;
- local notifications;
- OS integration;
- easier local credentials handling;
- desktop-specific UX.

Rule: shared domain logic should not live inside `src-tauri`. It should live in
Rust crates so Core, Node Daemon, CLI, and Tauri can reuse the same code.

## Repository Shape

Preliminary structure:

```text
crates/
  uprava-core/        shared domain model and contracts
  uprava-server/      Core Backend
  uprava-node/        Node Daemon
  uprava-client/      Rust API client
  uprava-tools/       tool/plugin contracts
  uprava-events/      event and trace contracts

apps/
  web/                React + Vite Web Control Panel
  cli/                Rust CLI
  desktop/            Tauri client later
```

This is not the final structure, but it reflects the main separation:

- Rust crates own system contracts and runtime;
- web app owns UI;
- desktop app is an optional shell;
- Core and Node remain separate deployable binaries.

## Deferred Decisions

- SQLx vs SeaORM vs another DB layer.
- SQLite-only first or immediate SQLite/Postgres abstraction.
- WebSocket vs SSE for event streams.
- OpenAPI vs custom generated client vs shared schema generation.
- Whether frontend lives under `apps/web` with Vite or later moves to Next.js.
- Whether Tauri appears in V01 as launcher or waits for a feature queue item.
- Exact package manager for frontend.
- Exact monorepo tooling for frontend.
- Exact Docker Compose service split for host-node and all-in-compose profiles.

## Current Recommendation

Initial stack:

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
Playwright
Rust tooling: cargo, rust-analyzer, rustfmt, clippy, bacon, nextest, audit, deny, taplo
```

V01 should avoid:

- Next.js as required app runtime;
- Node.js BFF layer;
- full workflow engine;
- full plugin marketplace;
- Tauri-specific domain logic.
