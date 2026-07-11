# Релизы Uprava

Статус: `active`

Current release baseline: `0.2.0`.

Этот ledger фиксирует implementation baselines. Он не заменяет
[`feature-queue.md`](feature-queue.md), где остается ранжированная очередь
future work.

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
| `0.2.0` | 2026-07-11 | current | Protocol v2 quality foundation, durable Core/Node state, workspace workbench and stable deployment paths |

## Current Baseline

`0.1.8` включает первый working distributed control panel, пять закрытых
feature queue slices после `0.1.0`, unified audit hardening slice и workspace
renderer/PTY terminal layer, а также первый deployable self-hosted release path:

- controlled-development security baseline;
- runtime/session hardening;
- stable workspace/reference model;
- read-only Project Workspace Inspector;
- workspace intervention layer с text save, bounded command runner, command
  history and diff/check entry points.
- quality gate honesty and Rust `1.88` MSRV alignment;
- Node allow-list enforcement, atomic local state writes, no-follow workspace
  writes and bounded command output during execution;
- ACK-after-reconnect command redispatch and session projection cursors для
  cross-scope event streams;
- visible web error states, send draft preservation and terminal enrollment
  status handling;
- healthcheck and logging failure hardening.
- Monaco-backed file editing and diff rendering в Web Control Panel;
- Core workspace terminal APIs для open, list, attach/stream, resize, input and
  close;
- Node Daemon PTY lifecycle management с workspace cwd enforcement,
  shell-profile policy, resize/input handling, status/exit frames and bounded
  replay;
- xterm.js terminal tabs with WebSocket attach, fit resize and interactive
  input/output streaming;
- retained bounded command runner для traceable controlled checks вроде
  `make l` and `make c`;
- GitHub Actions CI/CD для Core/Web images, Node artifact publishing, release
  manifest generation and deploy activation;
- server Make targets and scripts для release manifests, host Node artifact
  extraction, activation and deploy;
- self-hosted Codex adapter launch flags для noninteractive execution на
  production server, где effective boundary остается Unix user `uprava`,
  workspace allow-list and deployment ACLs.

Новые аудиты и temporary plans должны считать это фактами текущей реализации.
Они могут ссылаться на `V01`, когда обсуждают исторический первый продуктовый
срез.
