# Uprava Releases

Status: `active`

Current release baseline: `0.2.5`.

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
| `0.1.8` | 2026-07-08 | shipped | CI/CD deployment automation and self-hosted Codex execution posture |
| `0.2.0` | 2026-07-11 | shipped | Protocol v2 quality foundation, durable Core/Node state, workspace workbench and stable deployment paths |
| `0.2.1` | 2026-07-11 | shipped | Zarya 0.1 Web Control Panel alignment, flat work-sheet shell, system overview, agent work phases and visual regression gates |
| `0.2.2` | 2026-07-12 | shipped | Automatic main delivery, bounded CI workspaces, coordinated state epoch reset, scoped Node enrollment and functional production smoke |
| `0.2.3` | 2026-07-12 | shipped | Clean-bootstrap four-phase CI/CD, containerized prepare, explicit deploy/finalize boundaries and state-neutral delivery |
| `0.2.4` | 2026-07-12 | shipped | Delayed session messages: durable Core-owned one-off future turns with guarded dispatch |
| `0.2.5` | 2026-07-12 | current | Background Jobs and scheduled agent runs with observable per-run sessions and quota admission |

## Current Baseline

`0.2.5` includes the protocol-v2 `0.2.0` baseline, the completed Zarya 0.1 Web
UI/UX alignment and the clean-bootstrap four-phase delivery path. The current
implementation includes the first working
distributed control panel, the eight
completed feature queue slices after `0.1.0`, the unified audit hardening slice,
the workspace renderer/PTY terminal layer, and the first deployable self-hosted
release path:

- controlled-development security baseline;
- runtime/session hardening;
- stable workspace/reference model;
- read-only Project Workspace Inspector;
- workspace intervention layer with text save, bounded command runner, command
  history and diff/check entry points.
- daily-use hardening and deployment readiness across the sustained-use Web
  workbench and controlled self-hosted delivery path.
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
- a six-token monochrome Web Control Panel foundation with square geometry,
  semantic risk/notice roles and static enforcement against legacy raw styles;
- a flat App Shell, Dashboard system overview and runtime pipeline, phased
  Session/Agent Chat, 2.5D source/evidence disclosure and aligned Workspace
  Inspector chrome;
- component, keyboard and responsive visual regression coverage for the Zarya
  UI vocabulary.
- automatic immutable release publication and production activation after a
  successful update to `main`;
- bounded per-job CI workspaces with unconditional cleanup, orphaned-workspace GC
  and a free-space preflight;
- digest-pinned Core/Web/Node manifests, temporary registry credentials and
  project-scoped image/release retention;
- `prepare -> build -> deploy -> finalize` GitHub Actions phase boundaries with
  an immutable manifest artifact handoff;
- containerized source checks without a Docker socket and host-only build,
  deploy and finalize execution;
- clean host bootstrap from root-owned `/etc/uprava` inputs, state-neutral
  ordinary deploys and a read-only Node readiness interface;
- separate health/SHA/heartbeat/version finalization plus bounded Uprava-only
  release, image and stale runner-workspace retention.
- Core-owned delayed session-message records with UTC due time plus explicit
  IANA timezone, `scheduled -> sending -> sent | failed | cancelled` lifecycle,
  session-local UI, edit/reschedule/send-now/cancel/retry controls and typed
  guard failures; due dispatch uses the ordinary send-turn admission path.
- Core-owned paused Job definitions and observable Job Runs with retained
  effective-configuration snapshots, manual starts and interval/daily/weekly
  IANA schedules;
- atomic due-occurrence claim, restart-safe session correlation, one active run
  per Job with typed overlap skips, and stop-on-error by default;
- separate ordinary managed sessions for Job Runs, final provider message as
  summary, typed failures and links to the full session output/evidence;
- shared chat/Job provider-quota admission at the 5% threshold, audited force
  override and honest `unknown` when Codex exposes no reliable usage source;
- Web Job list/detail/run surfaces for creation, editing, schedule control,
  manual tests, force override, history, summaries and configuration snapshots.

New audits and temporary plans should treat these as current implementation
facts. They may still refer to `V01` when discussing the historical first
product cut.
