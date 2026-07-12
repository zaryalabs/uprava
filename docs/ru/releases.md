# Релизы Uprava

Статус: `active`

Current release baseline: `0.2.5`.

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
| `0.2.0` | 2026-07-11 | shipped | Protocol v2 quality foundation, durable Core/Node state, workspace workbench and stable deployment paths |
| `0.2.1` | 2026-07-11 | shipped | Zarya 0.1 Web Control Panel alignment, flat work-sheet shell, system overview, agent work phases and visual regression gates |
| `0.2.2` | 2026-07-12 | shipped | Automatic main delivery, bounded CI workspaces, coordinated state epoch reset, scoped Node enrollment and functional production smoke |
| `0.2.3` | 2026-07-12 | shipped | Clean-bootstrap four-phase CI/CD, containerized prepare, explicit deploy/finalize boundaries and state-neutral delivery |
| `0.2.4` | 2026-07-12 | shipped | Отложенные сообщения в сессии: долговечные Core-owned одноразовые будущие turn с guarded dispatch |
| `0.2.5` | 2026-07-12 | current | Background Jobs и scheduled agent runs с наблюдаемыми per-run sessions и quota admission |

## Current Baseline

`0.2.5` включает protocol-v2 baseline `0.2.0`, завершённое Zarya 0.1 Web UI/UX
alignment и clean-bootstrap four-phase delivery path. Текущая реализация включает
первый working distributed
control panel, восемь закрытых
feature queue slices после `0.1.0`, unified audit hardening slice и workspace
renderer/PTY terminal layer, а также первый deployable self-hosted release path:

- controlled-development security baseline;
- runtime/session hardening;
- stable workspace/reference model;
- read-only Project Workspace Inspector;
- workspace intervention layer с text save, bounded command runner, command
  history and diff/check entry points.
- daily-use hardening and deployment readiness для sustained-use Web workbench
  и controlled self-hosted delivery path.
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
- six-token monochrome foundation Web Control Panel с square geometry,
  semantic risk/notice roles и static enforcement против legacy raw styles;
- flat App Shell, Dashboard system overview and runtime pipeline, phased
  Session/Agent Chat, 2.5D source/evidence disclosure и согласованный chrome
  Workspace Inspector;
- component, keyboard and responsive visual regression coverage для Zarya UI
  vocabulary.
- automatic immutable release publication и production activation после
  успешного обновления `main`;
- bounded per-job CI workspaces с unconditional cleanup, orphaned-workspace GC и
  free-space preflight;
- digest-pinned Core/Web/Node manifests, temporary registry credentials и
  project-scoped image/release retention;
- phase boundaries GitHub Actions `prepare -> build -> deploy -> finalize` с
  immutable manifest artifact handoff;
- containerized source checks без Docker socket и host-only execution для
  build, deploy и finalize;
- clean host bootstrap из root-owned inputs `/etc/uprava`, state-neutral
  ordinary deploys и read-only interface Node readiness;
- отдельный health/SHA/heartbeat/version finalize и bounded Uprava-only
  retention releases, images и stale runner workspaces.
- Core-owned записи отложенных сообщений с UTC due time и явной IANA timezone,
  lifecycle `scheduled -> sending -> sent | failed | cancelled`, списком и UI
  внутри сессии, edit/reschedule/send-now/cancel/retry и typed guard failures;
  отправка в срок использует обычный send-turn admission path.
- Core-owned paused Job definitions и наблюдаемые Job Runs с сохранёнными
  snapshots effective configuration, manual starts и interval/daily/weekly IANA
  schedules;
- atomic due-occurrence claim, restart-safe session correlation, один active run
  на Job с typed overlap skips и stop-on-error по умолчанию;
- отдельные обычные managed sessions для Job Runs, final provider message как
  summary, typed failures и ссылки на полный session output/evidence;
- общая provider-quota admission для chat/Job с порогом 5%, audited force
  override и честным `unknown`, когда Codex не даёт надёжного usage source;
- Web surfaces Job list/detail/run для create/edit, schedule control, manual
  tests, force override, history, summaries и configuration snapshots.

Новые аудиты и temporary plans должны считать это фактами текущей реализации.
Они могут ссылаться на `V01`, когда обсуждают исторический первый продуктовый
срез.
