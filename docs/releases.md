# Релизы Uprava

Статус: `active`

Current release baseline: `0.2.14`.

Этот ledger фиксирует implementation baselines. Он не заменяет
[`feature-queue.md`](product/feature-queue.md), где остается ранжированная очередь
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
| `0.2.5` | 2026-07-12 | shipped | Background Jobs и scheduled agent runs с наблюдаемыми per-run sessions и quota admission |
| `0.2.6` | 2026-07-13 | shipped | Workspace-centered Web UI: Node/Workspace navigation и workspace Agent, Workbench, Jobs surfaces |
| `0.2.7` | 2026-07-19 | shipped | Causality/Trace UX, raw event/ref resolution и isolated structured Deduction |
| `0.2.8` | 2026-07-19 | shipped | Модульные Core/Node runtime boundaries, capability-oriented tests и автоматический architecture gate |
| `0.2.9` | 2026-07-19 | shipped | Прозрачный agent timeline: Conversation/Trace modes, сгруппированные live-события и stalled activity state |
| `0.2.10` | 2026-07-19 | shipped | Git-aware Review: branch/worktree snapshots, scoped diffs, risk signals и traceable check results |
| `0.2.11` | 2026-07-19 | shipped | Agent Tooling and Tool Registry v1: progressive Uprava MCP, scoped registry, ToolHive-backed Linear integration и traceable execution |
| `0.2.12` | 2026-07-19 | shipped | Plugin Registry v1: Core-owned lifecycle, manifest-driven Web Extension Host и bundled data-only Dark Theme |
| `0.2.13` | 2026-07-20 | shipped | CI visual baseline и SQLite migration reliability: синхронизированные Linux golden snapshots, per-connection busy timeout и изолированный concurrent migration test |
| `0.2.14` | 2026-07-20 | current | ToolHive Compose topology: отдельный pinned runtime, bounded host Node bridge, persistent OAuth state, smoke и ручной Linear acceptance runbook |

## Current Baseline

`0.2.14` включает baseline `0.2.13`, protocol-v2 baseline `0.2.0`, завершённое Zarya 0.1 Web UI/UX
alignment и clean-bootstrap four-phase delivery path. Текущая реализация включает
первый working distributed
control panel, четырнадцать implementation slices после `0.1.0`, unified audit
hardening slice и workspace
renderer/PTY terminal layer, а также первый deployable self-hosted release path:

- Linux visual regression baselines синхронизированы с pinned Playwright CI
  environment; SQLite busy timeout применяется к каждой Core connection, а
  concurrent migration test проверяет migration boundary без конкурирующего
  application bootstrap;

- Core-owned Plugin Registry отделён от Tool Registry и хранит immutable package
  metadata, installation state, compatibility, configuration revisions,
  permission grants и effective contribution projection;
- manifest-driven Web Extension Host применяет только валидированные typed
  `ui.theme` contributions; bundled data-only `uprava.theme-dark` переключает
  semantic tokens, Monaco и xterm с безопасным `core.light` fallback;
- Core-owned Tool Registry, permission-first progressive discovery
  `Search -> Inspect -> Execute` и session-scoped Uprava MCP leases;
- Node-owned observed capability inventory and desired/actual reconciliation;
  pinned ToolHive runtime вынесен в отдельный Compose service с bounded private
  bridge к host Node;
- Web management для integration lifecycle, availability и redacted tool-call
  trace; Linear OAuth URL передаётся только эфемерно и не сохраняется в durable
  state, audit или logs;
- Linear OAuth callback, discovery, read-only call и disconnect/reconnect
  подготовлены к ручной opt-in приёмке, но пока не являются подтверждённым E2E;
  `remote_revocation_confirmed = false` остаётся консервативным без upstream proof;

- coarse session trace с precision markers и типизированными
  source/evidence/cause/result/raw links;
- глобальный cursor-based event log, raw event detail и server-side reference
  resolver с явными unavailable states;
- workspace file/command/check/diff causality events, сохраняемые через
  Core-owned event log;
- isolated Deduction execution без resume live session, в ephemeral read-only
  Codex process со structured output schema;
- bounded evidence snapshots, Core-side ref allowlist/provenance validation,
  raw fallback, cancellation и explicit persistence в versioned
  `CausalityNarrative`;
- Web trace, aspect-based Context Inspector, raw event log и Deduction panel.
- Conversation и Trace разделены на URL-addressable режимы одной session
  surface; runtime bootstrap и события каждого agent turn сгруппированы в
  компактные раскрываемые блоки.
- Provider activity поступает в Core во время выполнения, а не только после
  завершения process; отсутствие новых событий у running turn становится
  видимым stalled attention state.
- Session SSE применяется как push-first read-model stream: каждый event
  немедленно обновляет session timeline, inventory summary, открытые trace,
  evidence и raw-event caches без каскада snapshot GET. Полные server
  projections остаются bootstrap/recovery boundary и обновляются только на
  значимых lifecycle/message границах.
- Public ingress разделяет authenticated UI, Node, stream, auth, enrollment и
  client-log buckets; UI/global budgets конфигурируются environment и имеют
  controlled-development defaults `600/5000` на минутное окно.
- Trace остаётся session-wide подробной летописью, Deduction и raw payload
  убраны на дополнительный уровень раскрытия, а reference actions используют
  явные inspect/copy controls.
- Core и Node composition roots отделены от application, transport,
  persistence, workspace, terminal и provider modules; прежние runtime и test
  монолиты разложены по capability boundaries без изменения protocol или state
  schema.
- Runtime architecture gate ограничивает рост composition roots и запрещает
  transport/process dependencies внутри persistence; logging policy рекурсивно
  проверяет все production-модули после декомпозиции.
- Node-owned Git snapshot различает branch, detached/unborn HEAD, primary и
  linked worktree, локальный upstream drift, staged/unstaged/untracked/conflict
  state и выполняющуюся Git operation; Core сохраняет snapshot на Placement и
  показывает warning для active runtime на том же repo/branch.
- Workspace Review предоставляет bounded `all`/`staged`/`unstaged` diff,
  per-file Monaco/raw preview, стабильные `WorkspaceDiff`/`DiffHunk` refs и
  долговечную историю traceable `make l`/`make c` check results.

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
- workspace-centered navigation с единственным глобальным Dashboard, деревом
  `Nodes -> Workspaces`, агрегированным Node Overview и независимым sidebar
  toggle;
- canonical workspace routes и compatibility redirects для session, Job и Job
  Run deep links с ownership guards и сохранением Inspector query;
- workspace Agent с session list, start flow и прежним lifecycle, Workbench с
  IDE-like grid `file tree | editor/diff` над PTY terminal и workspace-scoped
  Jobs list/create/detail/run;
- условный Context Inspector, который не резервирует колонку без reference и
  переходит в drawer на узком desktop;
- четыре Dashboard metrics, честная Recent Activity projection, единые status
  dimensions и unit/component/Playwright golden coverage desktop, narrow и
  mobile regression states.

Новые аудиты и temporary plans должны считать это фактами текущей реализации.
Они могут ссылаться на `V01`, когда обсуждают исторический первый продуктовый
срез.
