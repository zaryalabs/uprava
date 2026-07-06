# Contributing

This repository is moving from product and architecture design into implementation. The rules below keep the project easy to build, review, and extend while the codebase is still taking shape.

## Canonical Sources

- `README.md` explains the product direction.
- `docs/en/` is the canonical documentation tree.
- `docs/ru/` contains drafts, notes, and Russian-language working material.
- `AGENTS.md` contains short operational instructions for agents and contributors.
- `Makefile` is the gateway to local tooling.

When a technical or product decision changes, update the relevant document in `docs/en/`.

## Development Flow

1. Start from the current main branch.
2. Create a short-lived branch for the change.
3. Read the code and docs around the area you will touch.
4. Make a focused change.
5. Run the relevant local checks.
6. Run `make c` before commit, PR, or handoff.

Use branch names that make the work obvious:

```text
feat/core-node-registry
feat/web-session-view
fix/session-event-order
docs/architecture-boundaries
chore/tooling-precommit
```

## Commit Style

Use focused commits with concise Conventional Commit-style subjects:

```text
feat: add node heartbeat model
fix: preserve session event order
docs: clarify control-plane boundaries
chore: add pre-commit quality gate
```

Prefer a body when the reason is not obvious from the diff. Explain tradeoffs, migration notes, or follow-up work there.

## Lightweight Git Flow

- Keep branches small enough to review.
- Do not mix broad refactors with feature work.
- Do not commit generated caches, local secrets, or machine-specific files.
- Do not use `--no-verify` unless explicitly approved.
- Rebase or merge only when it keeps history clearer for the current collaboration.

## Architecture Principles

Uprava follows a domain-first architecture. Frameworks and transports support the product model; they should not define it.

Core domain boundaries:

- Core Backend is the control plane.
- Node Daemon is the data plane.
- Web Control Panel is a client of Core.
- Tool Registry and Plugin Registry live in Core.
- Node-local execution lives in Node Daemon.
- Trace, events, artifacts, permissions, and routing must be first-class system concepts.

Implementation guidance:

- Keep domain types and behavior independent from HTTP handlers, database rows, and UI components.
- Keep transport contracts explicit and versionable.
- Keep persistence details behind repository/service boundaries.
- Keep process/PTY/file operations inside node-side modules.
- Prefer small modules with clear ownership over generic utility layers.
- Add abstractions only when they reduce real duplication or protect a real boundary.

## Planned Repository Shape

The initial implementation should grow toward this shape:

```text
crates/
  uprava-domain/      domain model shared by Core, Node, and CLI
  uprava-protocol/    API/event contracts between Core, clients, and nodes
  uprava-core/        Core Backend
  uprava-node/        Node Daemon
  uprava-cli/         CLI
apps/
  web/                React + TypeScript + Vite web control panel
docs/
```

Names can change when implementation starts, but the control-plane/data-plane split should remain.

## Rust Standards

Expected baseline once the Rust workspace exists:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Preferred deeper tooling:

- `cargo-nextest` for workspace tests.
- `cargo audit` for vulnerability checks.
- `cargo deny` for dependency policy.
- `taplo-cli` for TOML formatting/linting.

Use `make` targets instead of calling these directly in routine workflows.

## Frontend Standards

The web app should follow `docs/en/tech-stack.md`:

- React 19 + TypeScript + Vite.
- Tailwind CSS v4.
- shadcn/ui conventions with local component ownership.
- lucide-react icons.
- TanStack Query for server state.
- TanStack Table for table-heavy views.
- React Hook Form + Zod for forms and validation.
- Vitest for unit/component logic.

Frontend checks should eventually cover:

- formatting;
- linting;
- TypeScript;
- tests;
- production build.

## Testing Expectations

Match test scope to risk:

- Domain logic needs focused unit tests.
- Protocol and persistence behavior need integration tests.
- Core-to-Node behavior needs contract or integration coverage.
- UI logic needs component/unit tests.
- Critical user workflows should eventually get Playwright coverage.

Tests should prove behavior at the boundary where the risk exists. Avoid tests that only mirror implementation details.

## Documentation Expectations

Update docs when a change affects:

- architecture;
- product behavior;
- setup or local workflow;
- command names;
- API/protocol contracts;
- quality gates.

Keep English docs canonical. Use Russian docs for drafting when useful, then stabilize the result in `docs/en/`.

## Local Quality Gate

Run:

```text
make c
```

before commit, PR, or handoff after code changes.

The gate should stay strict for implemented stacks and explicit about skipped stacks. During early setup, a target may no-op only when the corresponding files do not exist yet.
