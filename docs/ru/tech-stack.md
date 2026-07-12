# Uprava Tech Stack

Статус: `active`

Этот документ фиксирует предварительно выбранный технический стек для V01 и ближайших срезов feature queue.

## Короткое решение

V01 строим как Rust-first system with web-first UI:

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

Next.js 16 App Router пока не берем как базу V01. Причина не в том, что Next.js плохой, а в том, что у Uprava уже есть Rust Core Backend. В первой версии продукта не хочется создавать второй backend/BFF слой на Node.js без сильной причины.

## Архитектурная позиция

Core Backend and Node Daemon должны быть написаны на Rust.

Web UI должен быть обычным frontend-приложением, которое общается с Core API and event streams. На V01 его можно отдавать из Rust Core как static assets.

Tauri рассматриваем как desktop shell/client, но не как ядро продукта.

```text
Core Backend        Rust / Axum / Tokio
Node Daemon         Rust / Tokio
CLI                 Rust
Web Control Panel   React / TypeScript / Vite
Desktop Client      Tauri later, wraps Web Control Panel or talks to Core
```

## Local development environment

Docker Compose - canonical local Core/Web bootstrap and smoke-test environment
для V01 development. Это не production deployment model, а инструмент
стабильности: browser-facing Core/Web startup, hardened local auth and basic
diagnostics должны воспроизводимо стартовать на каждой машине. Node-facing
protocol paths идут через host Node Daemon, когда нужен реальный workspace or
provider access.

Базовый Compose setup должен давать:

- predictable ports для Core and Web;
- persistent but resettable SQLite/Core state volumes;
- hardened Core/Web smoke path, который работает без Codex;
- host Node Daemon path для реальных local workspaces and host credentials;
- health checks, useful for `make`, Playwright and CI;
- documented reset and log-collection commands.

Для реального контроля local workspace может понадобиться Node Daemon на host.
Compose остается стабильным способом стартовать Core/Web and infrastructure
smoke path, while `make node-r` and `make codex-smoke` cover host Node
enrollment, workspace access and real provider execution там, где Codex
установлен.

## Rust stack

Текущий implementation baseline использует Rust `1.88` как minimum supported
Rust version. Это следует за locked dependency graph вместо pinning transitive
dependencies к более старому provisional toolchain.

### Core Backend

Предварительный выбор:

- Rust;
- Tokio;
- Axum;
- Serde;
- SQLx with numbered, checksummed migrations;
- SQLite for local/single-user first;
- Postgres-compatible architecture later;
- WebSocket or SSE for live session/events;
- structured logging/tracing.

Core Backend отвечает за:

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

Предварительный выбор:

- Rust;
- Tokio;
- outbound connection to Core;
- local workspace management;
- process lifecycle management;
- persistent agent sessions;
- output/event streaming;
- workspace binding validation for V01;
- workspace file operations;
- PTY/terminal lifecycle for interactive workspace terminals;
- local tool execution in later Tool Registry slices.

Node Daemon должен быть максимально переносимым. V01 ориентируется на desktop/server nodes, но архитектура должна не блокировать future cloud nodes, sandboxes and microVM hosts.

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

Предварительный CI/dependency hygiene baseline:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo audit
cargo deny check
taplo fmt --check
```

`cargo audit` and `cargo deny` не заменяют друг друга. `audit` отвечает за known security advisories, `deny` - за broader dependency policy: licenses, bans, advisories, duplicated crates and sources.

Репозиторий хранит dependency policy в `deny.toml`, а TOML formatting policy в
`taplo.toml`. `make init` installs `cargo-audit`, `cargo-deny` and `taplo-cli`
if they are missing; `make c` requires them through `rust-dl`. Taplo is
currently used for formatting checks only.

## Frontend stack

### npm audit policy для 0.2.0

Release CI отклоняет moderate, high и critical production advisories. Monaco
для 0.2.0 закреплен на `0.53.0`, поскольку более новая проверенная ветка
объявляет уязвимую зависимость DOMPurify. В development-only Vite graph остается
один low-severity advisory `esbuild`: для эксплуатации нужен локальный Windows
user с запущенным development server, а в Linux static production image этот
код не попадает. Для `0.2.2` exception перепроверен: он остаётся
development-only, а production audit по-прежнему отклоняет moderate и более
серьёзные advisories. Owner: Uprava maintainers. Next expiry: `0.2.3` или
2026-08-31 — что наступит раньше; Vite нужно обновить, когда совместимый graph
получит исправленный esbuild.

### Base

Выбор для V01:

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

### Почему Vite SPA, а не Next.js на V01

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

Это хорошо совпадает с Uprava:

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

Realtime updates на V01 можно строить так:

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
- events;
- future tools/plugins registry;
- future artifacts;
- future review queues;
- future task runs.

### Forms and validation

React Hook Form + Zod нужны для:

- project settings;
- node setup;
- session launch forms;
- future tool/plugin configuration;
- future integration credentials forms;
- future task run forms.

Для protocol v2 Rust schema roots в `uprava-protocol` являются source of truth.
Из них генерируются tracked JSON Schema, TypeScript types and Ajv runtime
validators для Web-facing HTTP, SSE and terminal payloads. Node control-only
roots не попадают в browser bundle, а generated artifacts проверяются на drift.

### Testing

V01:

- Vitest для unit/component logic;
- Rust tests для core/node crates.
- Playwright для automated Web Control Panel E2E tests against the Docker
  Compose local profile;
- Playwright CLI для agent/operator UI verification during implementation:
  inspect running app, click through flows, capture screenshots and confirm
  state is visible outside assistant text.

Later:

- integration tests для Core <-> Node Daemon protocol;
- scenario/eval tests for agent workflows.

Playwright CLI mode не заменяет deterministic E2E coverage. Это interactive
verification path for agents перед handoff, когда UI change требует visual or
workflow confirmation.

## Tauri

Tauri v2 не является foundation для V01, но остается сильным кандидатом для desktop client.

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

Это не финальная структура, но она отражает главное разделение:

- Rust crates own system contracts and runtime;
- web app owns UI;
- desktop app is optional shell;
- Core and Node remain separate deployable binaries.

## Deferred decisions

- SQLite-only first or immediate SQLite/Postgres abstraction.
- WebSocket vs SSE for event streams.
- Whether frontend lives under `apps/web` with Vite or later moves to Next.js.
- Whether Tauri appears in V01 as launcher or waits for a feature queue item.
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
