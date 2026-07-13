# Uprava 0.2.0 polish handoff

Status: historical

Updated: 2026-07-13

This directory is the tracked, portable historical context package for the
completed `0.2.0` quality-foundation work. It contains context that was
previously unique to ignored `.local/` files. Temporary plans used during the
release were removed after shipping; durable outcomes now live in canonical
documents and the release ledger.

## Read order

1. Repository [README](../../README.md) and [AGENTS.md](../../AGENTS.md).
2. [Portable handoff](0.2.0-handoff.md) for the current branch state, checks,
   constraints and resume procedure.
3. [M7 checkpoint handoff](0.2.0-checkpoint-2026-07-10-m7.md) for the current
   pause point and next recommended implementation slice.
4. [Implementation ledger](0.2.0-implementation-ledger.md) when a milestone
   needs historical rationale, commit evidence or its accepted residuals.
5. [2026-07-09 audit](../audit/audit-2026-07-09.md) for the original quality
   rationale and finding baseline.
6. [Release ledger](../releases.md) for the shipped result.

Canonical product and operational context:

- [Architecture](../systems/architecture.md)
- [2026-07-09 audit](../audit/audit-2026-07-09.md)
- [Feature queue](../product/feature-queue.md)
- [Tech stack](../development/tech-stack.md)
- [Versioning](../versioning.md)
- [Release ledger](../releases.md)
- [Local development runbook](../runbooks/v01-local-dev.md)
- [Deployment](../deploy/deployment.md)
- [Deployment observability](../deploy/deployment-observability.md)

The canonical product and architecture documentation is maintained in Russian
under [`docs/`](../README.md).

## What moved out of ignored local context

| Ignored source | Tracked replacement |
| --- | --- |
| `.local/0.2.0-handoff.md` | [Portable handoff](0.2.0-handoff.md) |
| `.local/context/0.2.0-execution.md` | [Implementation ledger](0.2.0-implementation-ledger.md) and [portable handoff](0.2.0-handoff.md) |
| `.local/context/README.md` | The only relevant instruction was to read private context when requested; this tracked index replaces that routing for `0.2.0` |

The following ignored files are intentionally not copied:

- `.local/state/**`, SQLite WAL/SHM files and `.local/logs/**` are
  machine-local runtime data and may contain credentials or user content;
- platform task/thread identifiers and worker/reviewer orchestration are not
  needed by the new one-stream execution mode;
- `.local/history/v0.1-*` is superseded by the tracked release ledger,
  canonical docs and Git history;
- older audit drafts under `.local/history/` are superseded by the tracked
  2026-07-09 audit and release ledger;
- local protocol, Playwright and Codex notes are represented by the current
  code, tests, canonical system-direction docs and the implementation ledger.

Do not add credentials, runtime databases, logs, provider transcripts or
machine-specific release artifacts to this directory.

## Maintenance rule

Do not resume this package as an active plan. Current future work belongs in the
feature queue and current temporary plans. Update these files only to preserve
historical accuracy or repair references; durable product and architecture
decisions belong in the canonical Russian documents under `docs/`.
