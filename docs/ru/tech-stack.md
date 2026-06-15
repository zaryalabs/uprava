# Cortex Tech Stack

Статус: `draft`

Этот документ фиксирует предварительно выбранный технический стек для Stage 1 и ближайших стадий.

## Короткое решение

Stage 1 строим как Rust-first system with web-first UI:

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

Next.js 16 App Router пока не берем как базу Stage 1. Причина не в том, что Next.js плохой, а в том, что у Cortex уже есть Rust Core Backend. На первой стадии не хочется создавать второй backend/BFF слой на Node.js без сильной причины.

## Архитектурная позиция

Core Backend and Node Daemon должны быть написаны на Rust.

Web UI должен быть обычным frontend-приложением, которое общается с Core API and event streams. На Stage 1 его можно отдавать из Rust Core как static assets.

Tauri рассматриваем как desktop shell/client, но не как ядро продукта.

```text
Core Backend        Rust / Axum / Tokio
Node Daemon         Rust / Tokio
CLI                 Rust
Web Control Panel   React / TypeScript / Vite
Desktop Client      Tauri later, wraps Web Control Panel or talks to Core
```

## Rust stack

### Core Backend

Предварительный выбор:

- Rust;
- Tokio;
- Axum;
- Serde;
- SQLx or SeaORM later, decision deferred;
- SQLite for local/single-user first;
- Postgres-compatible architecture later;
- WebSocket or SSE for live session/events;
- structured logging/tracing.

Core Backend отвечает за:

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

Предварительный выбор:

- Rust;
- Tokio;
- outbound connection to Core;
- local workspace management;
- PTY/terminal support;
- process lifecycle management;
- file operations;
- persistent agent sessions;
- output/event streaming;
- local tool execution.

Node Daemon должен быть максимально переносимым. Stage 1 ориентируется на desktop/server nodes, но архитектура должна не блокировать future cloud nodes, sandboxes and microVM hosts.

### CLI

CLI тоже лучше писать на Rust, чтобы переиспользовать shared crates and API client.

Возможные задачи CLI:

- start local Core;
- register Node;
- inspect nodes/sessions;
- connect to Core;
- run diagnostics;
- manage plugins/tools later.

### Rust tooling

Базовый Rust tooling:

- `cargo` - основной build/test/package tool.
- `rust-analyzer` - обязательный language server для разработки.
- `rustfmt` - единый формат кода.
- `cargo clippy` - linting and correctness checks.
- `bacon` - локальный watcher для быстрого dev loop.
- `cargo-nextest` - основной test runner для workspace tests.
- `cargo audit` - проверка known vulnerabilities в dependency tree.
- `cargo deny` - licenses, advisories, duplicate dependencies and dependency policy.
- `taplo-cli` - форматирование/проверка TOML файлов.

Предварительный local dev loop:

```text
cargo fmt
cargo clippy --workspace --all-targets
cargo nextest run --workspace
```

Для ежедневной разработки можно использовать `bacon`, чтобы постоянно гонять `check`, `clippy` or targeted tests во время изменения кода.

Предварительный CI/security baseline:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo audit
cargo deny check
taplo fmt --check
taplo lint
```

`cargo audit` and `cargo deny` не заменяют друг друга. `audit` отвечает за known security advisories, `deny` - за broader dependency policy: licenses, bans, advisories, duplicated crates and sources.

## Frontend stack

### Base

Выбор для Stage 1:

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

### Почему Vite SPA, а не Next.js на Stage 1

Vite SPA проще для текущей архитектуры:

- Core Backend уже есть в Rust;
- web app можно отдавать как static assets из Core;
- проще self-hosting and local single-user deployment;
- проще завернуть в Tauri позже;
- меньше риска размыть ответственность между Rust Core and Node.js BFF;
- realtime UI все равно будет жить вокруг Core API, WebSocket/SSE and client state.

Next.js может вернуться позже, если появится один из факторов:

- отдельный cloud/web frontend deployment;
- нужен BFF layer for web;
- нужны Server Components/Server Actions как сильное преимущество;
- появляются public/marketing/docs pages with SSR/SEO needs;
- появляется multi-tenant SaaS frontend, где Next.js дает реальную пользу.

### UI conventions

shadcn/ui берем как convention and component source model, а не как внешний black-box component library.

Это хорошо совпадает с Cortex:

- компоненты лежат в коде проекта;
- их можно адаптировать под продукт;
- API компонентов предсказуемы для людей and AI agents;
- удобно строить собственную design system;
- можно добавлять custom workbench components поверх base primitives.

lucide-react используем как default icon set.

### State and data fetching

TanStack Query - default для server state:

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

Realtime updates на Stage 1 можно строить так:

```text
HTTP queries for snapshots
WebSocket/SSE for events
TanStack Query cache updates from event stream
```

Не стоит тащить global client state manager до появления явной необходимости. Local component state + URL state + TanStack Query should be enough.

### Tables

TanStack Table нужен для:

- nodes list;
- sessions list;
- tools/plugins registry;
- artifacts;
- events;
- future review queues;
- future task runs.

### Forms and validation

React Hook Form + Zod нужны для:

- project settings;
- node setup;
- tool/plugin configuration;
- integration credentials forms;
- session launch forms;
- future task run forms.

Zod полезен как frontend validation boundary. Backend contracts все равно должны быть Rust-first; позже можно подумать о generated schemas.

### Testing

Stage 1:

- Vitest для unit/component logic;
- Rust tests для core/node crates.

Later:

- Playwright для Web Control Panel e2e;
- integration tests для Core <-> Node Daemon protocol;
- scenario/eval tests for agent workflows.

## Tauri

Tauri v2 не является foundation для Stage 1, но остается сильным кандидатом для desktop client.

Возможные роли Tauri:

- desktop shell around Web Control Panel;
- local launcher for Core + Node Daemon;
- tray app;
- local notifications;
- OS integration;
- easier local credentials handling;
- desktop-specific UX.

Правило: shared domain logic не должна жить внутри `src-tauri`. Ее нужно держать в Rust crates, чтобы Core, Node Daemon, CLI and Tauri могли переиспользовать один код.

## Repository shape

Предварительная структура:

```text
crates/
  cortex-core/        shared domain model and contracts
  cortex-server/      Core Backend
  cortex-node/        Node Daemon
  cortex-client/      Rust API client
  cortex-tools/       tool/plugin contracts
  cortex-events/      event and trace contracts

apps/
  web/                React + Vite Web Control Panel
  cli/                Rust CLI
  desktop/            Tauri client later
```

Это не финальная структура, но она отражает главное разделение:

- Rust crates own system contracts and runtime;
- web app owns UI;
- desktop app is optional shell;
- Core and Node remain separate deployable binaries.

## Deferred decisions

- SQLx vs SeaORM vs another DB layer.
- SQLite-only first or immediate SQLite/Postgres abstraction.
- WebSocket vs SSE for event streams.
- OpenAPI vs custom generated client vs shared schema generation.
- Whether frontend lives under `apps/web` with Vite or later moves to Next.js.
- Whether Tauri appears in Stage 1 as launcher or waits until Stage 2/3.
- Exact package manager for frontend.
- Exact monorepo tooling for frontend.

## Current recommendation

Начальный стек:

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
