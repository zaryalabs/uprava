# Uprava 0.2.0 polish handoff

Status: active

Updated: 2026-07-10

This directory is the tracked, portable context package for continuing the
`0.2.0` quality-foundation work on another machine. It contains only context
that was previously unique to ignored `.local/` files. Existing tracked plans
and canonical documents are linked rather than copied.

## Read order

1. Repository [README](../../README.md) and [AGENTS.md](../../AGENTS.md).
2. [Portable handoff](0.2.0-handoff.md) for the current branch state, checks,
   constraints and resume procedure.
3. [Completion plan](../tmp-plans/0.2.0-completion-from-current.md) for the
   sequential implementation order from the current state to final `0.2.0`.
4. [Implementation ledger](0.2.0-implementation-ledger.md) when a milestone
   needs historical rationale, commit evidence or its accepted residuals.
5. [Quality-foundation plan](../tmp-plans/0.2.0-quality-foundation.md) for the
   authoritative scope, finding matrix, non-goals and final release criteria.
6. [RC checklist](../tmp-plans/0.2.0-rc-checklist.md) for release evidence.

Canonical product and operational context:

- [Architecture](../en/architecture.md)
- [2026-07-09 audit](../en/audit/audit-2026-07-09.md)
- [Feature queue](../en/feature-queue.md)
- [Tech stack](../en/tech-stack.md)
- [Versioning](../en/versioning.md)
- [Release ledger](../en/releases.md)
- [Local development runbook](../en/runbooks/v01-local-dev.md)
- [Deployment](../en/deployment.md)
- [Deployment observability](../en/deployment-observability.md)

The Russian canonical mirrors live at the equivalent paths under
[`docs/ru`](../ru/README.md).

## What moved out of ignored local context

| Ignored source | Tracked replacement |
| --- | --- |
| `.local/0.2.0-handoff.md` | [Portable handoff](0.2.0-handoff.md) |
| `.local/context/0.2.0-execution.md` | [Implementation ledger](0.2.0-implementation-ledger.md) plus the [completion plan](../tmp-plans/0.2.0-completion-from-current.md) |
| `.local/context/README.md` | The only relevant instruction was to read private context when requested; this tracked index replaces that routing for `0.2.0` |

The following ignored files are intentionally not copied:

- `.local/state/**`, SQLite WAL/SHM files and `.local/logs/**` are
  machine-local runtime data and may contain credentials or user content;
- platform task/thread identifiers and worker/reviewer orchestration are not
  needed by the new one-stream execution mode;
- `.local/history/v0.1-*` is superseded by the tracked release ledger,
  canonical docs and Git history;
- older audit drafts under `.local/history/` are superseded by the tracked
  2026-07-09 audit and quality-foundation plan;
- local protocol, Playwright and Codex notes are represented by the current
  code, tests, canonical design docs and the implementation ledger.

Do not add credentials, runtime databases, logs, provider transcripts or
machine-specific release artifacts to this directory.

## Maintenance rule

Update the portable handoff and implementation ledger at meaningful milestone
boundaries. Keep detailed future work in the completion plan and durable
product/architecture decisions in synchronized `docs/en` and `docs/ru`
documents.
