# Uprava Releases

Status: `active`

Current release baseline: `0.1.8`.

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
| `0.1.5` | 2026-07-06 | shipped | Workspace intervention layer |
| `0.1.6` | 2026-07-06 | shipped | Unified audit hardening |
| `0.1.7` | 2026-07-06 | shipped | Workspace renderer and PTY terminal layer |
| `0.1.8` | 2026-07-08 | current | CI/CD deployment automation and self-hosted Codex execution posture |

## Current Baseline

`0.1.8` includes the first working distributed control panel, the five
completed feature queue slices after `0.1.0`, the unified audit hardening slice,
the workspace renderer/PTY terminal layer, and the first deployable self-hosted
release path:

- controlled-development security baseline;
- runtime/session hardening;
- stable workspace/reference model;
- read-only Project Workspace Inspector;
- workspace intervention layer with text save, bounded command runner, command
  history and diff/check entry points.
- quality gate honesty and Rust `1.88` MSRV alignment;
- Node allow-list enforcement, atomic local state writes, no-follow workspace
  writes and bounded command output during execution;
- ACK-after-reconnect command redispatch and session projection cursors for
  cross-scope event streams;
- visible web error states, send draft preservation and terminal enrollment
  status handling;
- healthcheck and logging failure hardening.
- Monaco-backed file editing and diff rendering in the Web Control Panel;
- Core workspace terminal APIs for open, list, attach/stream, resize, input and
  close;
- Node Daemon PTY lifecycle management with workspace cwd enforcement,
  shell-profile policy, resize/input handling, status/exit frames and bounded
  replay;
- xterm.js terminal tabs with WebSocket attach, fit resize and interactive
  input/output streaming;
- retained bounded command runner for traceable controlled checks such as
  `make l` and `make c`;
- GitHub Actions CI/CD for Core/Web images, Node artifact publishing, release
  manifest generation and deploy activation;
- server Make targets and scripts for release manifests, host Node artifact
  extraction, activation and deploy;
- self-hosted Codex adapter launch flags for noninteractive execution on the
  production server while the effective boundary remains the `uprava` Unix user,
  workspace allow-list and deployment ACLs.

New audits and temporary plans should treat these as current implementation
facts. They may still refer to `V01` when discussing the historical first
product cut.
