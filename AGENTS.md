# AGENTS.md

Quick guide for agents working in this repository.

> [!IMPORTANT]
> If `./.local/context/` exists, read `./.local/context/README.md` before starting work. `.local/` is private local context and is not committed.

## Start Here

- Read `README.md` first.
- Treat `docs/en/` as the canonical product and architecture documentation.
- Use `docs/ru/` for drafts, source notes, and Russian-language thinking when useful.
- Keep architecture and process decisions in `docs/`; keep this file short and operational.

## Commands

- Use `make` for routine project operations.
- Run `make help` to see available commands.
- Run `make c` before commits or handoff after code changes.
- Run `make l` for a faster local check while iterating.

## Project Shape

- `docs/` - product, architecture, roadmap, and stack documentation.
- `crates/` - planned Rust workspace crates.
- `apps/web/` - planned React + TypeScript + Vite web control panel.
- `Makefile` - gateway to local tooling.
- `.pre-commit-config.yaml` - commit-time quality gates.

The repository is currently in the transition from documentation/design to implementation. Tooling must tolerate missing code directories until the implementation scaffold exists.

## Technical Direction

Follow the stack documented in `docs/en/tech-stack.md`:

- Rust workspace for Core Backend, Node Daemon, CLI, domain, and protocol code.
- Axum/Tokio for backend services.
- SQLite first, with Postgres-compatible architecture later.
- React 19 + TypeScript + Vite for the web control panel.
- Tailwind CSS v4, shadcn/ui conventions, lucide-react.
- TanStack Query/Table, React Hook Form, Zod, Vitest.

## Architecture Rules

- Core Backend is the control plane.
- Node Daemon is the data plane.
- Clients talk to Core; Core routes commands and state to nodes.
- Core owns projects, nodes, sessions, event log, trace metadata, Tool Registry, Plugin Registry, permissions, and routing.
- Node Daemon owns local workspaces, files, PTY/process lifecycle, local tool execution, and agent process management.
- Do not make the web client depend on direct access to every node.
- Prefer DDD style in code;
- Do not hide integration behavior behind untraced agent text when it should become a tool, event, artifact, or visual block.

## Code Conventions

- Prefer explicit domain boundaries over framework-driven structure.
- Keep transport, persistence, UI, and domain logic separated.
- Keep changes focused; avoid unrelated refactors.
- Update docs when architecture, workflow, or product behavior changes.
- Do not bypass pre-commit hooks with `--no-verify` unless explicitly instructed.

## Quality Gate

Before commit or handoff:

1. Run `make c`.
2. Fix failures instead of weakening checks.
3. If a check cannot run because the relevant stack is not scaffolded yet, leave the Makefile target as a clear no-op with a message.
