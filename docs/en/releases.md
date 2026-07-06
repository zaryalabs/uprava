# Uprava Releases

Status: `active`

Current release baseline: `0.1.5`.

This ledger records implementation baselines. It does not replace
[`feature-queue.md`](feature-queue.md), which remains the ranked list of future
work.

## Release Ledger

| Version | Date | Status | Completed Slice |
| --- | --- | --- | --- |
| `0.1.0` | 2026-07-06 | shipped | V01 Distributed Agent Control Panel |
| `0.1.1` | 2026-07-06 | shipped | Security baseline |
| `0.1.2` | 2026-07-06 | shipped | Runtime/session hardening |
| `0.1.3` | 2026-07-06 | shipped | Workspace shell and reference model |
| `0.1.4` | 2026-07-06 | shipped | Read-only Project Workspace Inspector |
| `0.1.5` | 2026-07-06 | current | Workspace intervention layer |

## Current Baseline

`0.1.5` includes the first working distributed control panel plus the five
completed feature queue slices after `0.1.0`:

- controlled-development security baseline;
- runtime/session hardening;
- stable workspace/reference model;
- read-only Project Workspace Inspector;
- workspace intervention layer with text save, bounded command runner, command
  history and diff/check entry points.

New audits and temporary plans should treat these as current implementation
facts. They may still refer to `V01` when discussing the historical first
product cut.
