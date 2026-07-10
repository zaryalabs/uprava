# Uprava Feature Queue

Status: `active`

This document uses an implementation queue instead of a phase-based roadmap.

The queue is not a calendar, milestone ladder, or delivery promise. It is a
ranked set of product and architecture slices ordered by dependency, complexity,
risk, and value. Items can move as the design sharpens.

## Queue Rules

Each queue item should capture:

- **Value** - why this matters to the user or to Uprava as a system.
- **Dependency** - what must exist first.
- **Complexity** - implementation difficulty and surface area.
- **Risk** - unknowns, security concerns, or product ambiguity.
- **First useful slice** - the smallest version worth building.
- **Target direction** - how the mechanism should grow without overfitting the
  first implementation.

Use this document to answer:

```text
What should we build next, and why this before that?
```

Do not use it to answer:

```text
What does the first version contain?
```

That belongs in [v01.md](v01.md).

## Queue Overview

Current release baseline: `0.1.8`. Done items `0` through `5`, the unified
audit hardening release, the `5a` workspace renderer release and the first
self-hosted CI/CD deployment baseline correspond to the shipped versions
recorded in [`releases.md`](releases.md).
The next planned queue item is a daily-use hardening and deployment readiness
slice before new product mechanisms are added.

| Order | Done | Mechanism / Feature Slice | First Useful Slice | Dependency | Complexity |
| --- | --- | --- | --- | --- | --- |
| 0 | + | V01 Distributed Agent Control Panel | Multi-node chat/session control panel | Current design baseline | High |
| 1 | + | Security baseline | Trusted-dev warning, node auth, local web auth, credential handling, audit minimum | V01 control path | High |
| 2 | + | Runtime/session hardening | Robust lifecycle, resume, stop, blocked, stale states | V01 runtime path | Medium |
| 3 | + | Workspace shell and reference model | Stable refs and routes for future workspace evidence | V01 entity/session model | Medium |
| 4 | + | Read-only Project Workspace Inspector | File tree, metadata, safe text viewer | Workspace refs, Node file reads | Medium |
| 5 | + | Workspace intervention layer | Lightweight editor, terminal, command history, diff/check entry points | Read-only inspector, events | High |
| 5a | + | Workspace renderer and PTY terminal layer | Monaco file/diff renderers and xterm-backed interactive PTY sessions | Workspace intervention, Core/Node control channel | High |
| 6 | - | Daily-use hardening and deployment readiness | Stable panel layout, product polish, server deploy path, CI/CD baseline | `0.1.8` deployable workbench, security baseline | High |
| 6a | - | Provider-native persistent execution policy | Safe provider defaults, explicit unsafe mode, real approvals and visible effective policy | 0.2.0 quality foundation, provider-native persistent runtime | Very high |
| 7 | - | Causality and trace UX | Coarse source/cause links with raw fallback | Workspace refs, event log | Medium |
| 8 | - | Git and review basics | Better diff, branch/worktree awareness, check results | Workspace intervention, trace | Medium |
| 9 | - | Tool Registry v1 | Real tool metadata, permissions, routing, and audit policy | V01 capability model, events | High |
| 10 | - | Plugin Registry v1 | Installed plugin metadata, configuration, exposed tools, and artifact types | Tool Registry v1 | High |
| 11 | - | First external integrations | Git provider and task tracker integration slices | Tool/Plugin Registry | High |
| 12 | - | Visual artifact system | Test reports, richer diffs, timelines, dashboards/forms as first-class artifacts | Trace, registry contracts | High |
| 13 | - | Dynamic UI from agents | Schema/tool/plugin-rendered UI with safe fallbacks | Visual artifact system, plugins | High |
| 14 | - | Task-based sandbox runtime | Bounded run contract, isolated workspace, expected evidence | Runtime, workspace, trace | Very high |
| 15 | - | Hybrid managed sessions | Persistent session can spawn bounded runs and merge evidence back | Task runtime | Very high |
| 16 | - | Team/cloud model | Users, roles, shared projects, managed Core/nodes | Mature personal workflow | Very high |
| 17 | - | Beyond software development | Research, analytics, documents, finance, knowledge workflows | Mature artifact/plugin model | Very high |
| 18 | - | Audit follow-up refactors | Core/Node module split, generated protocol contracts, async workspace command API | `0.1.6` audit hardening | Medium |

## Queue Details

### 0. V01 Distributed Agent Control Panel

**Value:** Gives the first tactile product: a user can run Core, connect one or
more nodes, bind projects/workspaces, start persistent Codex-backed sessions,
and control those sessions from a web UI.

**First useful slice:** Defined in [v01.md](v01.md).

**Target direction:** Keep the first product small while preserving the system
model for workspaces, providers, tools, plugins, visual artifacts, task runs,
mobile, and team/cloud modes.

### 1. Security baseline

**Value:** Makes the V01 control path safe enough to use beyond a purely trusted
local prototype without pretending to solve full team/cloud security.

**First useful slice:** Explicit deployment profiles, visible non-production
warning until hardened mode is enabled, node enrollment/auth, credential storage
rules, revoke/rotate basics, local web auth/session handling, origin/CSRF checks
where relevant, token redaction, and minimal security/audit events.

**Current implementation note:** `controlled_dev` with `UPRAVA_WEB_AUTH=auto`
is the supported V01 profile. It enables local password setup/login, session
and CSRF cookies, protected browser routes, origin checks, node bearer
credentials for heartbeat/control, node revoke/rotate, private Node state-file
permissions where supported, token redaction and minimal
`security_audit_events` records. `local_trusted`, disabled browser auth and
auto-approved enrollment are rejected at startup.

**Target direction:** Grow into permissions, secrets handling, stronger audit,
mTLS or request signing, keychain-backed credentials, team RBAC, and managed
cloud security without changing the Core/Node responsibility split.

### 2. Runtime/session hardening

**Value:** Makes live agent work feel reliable instead of like a wrapped CLI.

**First useful slice:** Clear lifecycle states, explicit expiry/resume behavior,
blocked approvals, interrupt/stop semantics, stale node handling, and degraded
resume messaging.

**Current implementation note:** Core and Node now persist and project
start/ready/running/blocked/resuming/stopped/error/expired runtime state,
bounded provider resume refs, idle expiry, stale/offline/revoked node warnings,
detached-session gates, approval request/resolution state and command preflight.
The Web Control Panel and agent projection only advertise send-turn and
approval-resolution commands when those commands match Core runtime/session
preflight, and resolved historical approval blocks no longer expose approval
actions.

**Target direction:** Support multiple runtime strategies and provider adapters
without changing Core/UI concepts.

### 3. Workspace shell and reference model

**Value:** Lets future chat, trace, artifacts, review, and agents point at the
same workspace evidence without forcing the full inspector into V01.

**First useful slice:** Stable ids, routes, and reference shapes for project,
workspace, session, turn, message, runtime event, and reserved future workspace
objects such as file, file range, edit, terminal session, command, output range,
diff hunk, check result, artifact, and trace event.

**Current implementation note:** Shared Rust and Web protocol contracts now
define stable Uprava refs for project, placement, workspace, session, runtime,
turn, message, block, artifact, event, command, approval, warning, tool call,
file/file range, terminal/command/output range, diff hunk, check result,
workspace edit, trace event, external entity, and unknown future refs. The Web
Control Panel has stable project, workspace, placement, node, and session route
helpers, a project route, a workspace route alias, inspector stack URL encoding,
and explicit fallback handling for reserved future workspace refs.

**Target direction:** Shared addressability for UI navigation, agent prompts,
review decisions, plugin blocks, and task-run packages.

### 4. Read-only Project Workspace Inspector

**Value:** Lets the user see where the agent is working before Uprava adds
direct intervention tools.

**First useful slice:** Workspace file tree, file metadata, safe text file
viewer, readable states for large/binary/ignored/generated/permission-denied
files, and node-side workspace boundary enforcement.

**Current implementation note:** Core exposes authenticated placement workspace
tree and file-read routes, dispatching read-only commands to the Node Daemon and
waiting for typed command results. The Node Daemon normalizes relative paths,
enforces workspace and allowed-root boundaries, avoids symlink traversal, caps
tree and text reads, and returns explicit states for large, binary, generated,
ignored, missing, symlink, and permission-denied paths. The Web Control Panel
mounts a file tree and safe text viewer on workspace routes.

**Target direction:** A project surface that can later host editor, terminal,
diff, checks, artifacts, and trace links.

### 5. Workspace intervention layer

**Value:** Gives the human narrow control when direct action is faster than
asking the agent to describe or fix its own environment.

**First useful slice:** Controlled text writes or patch applies, workspace
terminal/PTY or command runner, command/output history, session-level diff, and
basic check/test entry points.

**Current implementation note:** The first intervention slice now extends the
Project Workspace Inspector with explicit text-file save semantics, a bounded
workspace command runner, command/check result display, persisted command result
history, and a git diff snapshot entry point. Core routes these actions through
placement-scoped commands and persists command-result payloads; Node enforces
allowed workspace roots, path normalization, protected generated/ignored paths,
text-size caps, no-shell command execution, timeout limits, and bounded output.
The Web Control Panel exposes save, `make l`, `make c`, custom command, diff and
history controls in the workspace surface.

**Target direction:** Lightweight developer workbench ergonomics without
becoming a full browser IDE.

### 5a. Workspace renderer and PTY terminal layer

**Value:** Makes the workspace surface behave like a real developer workbench:
Monaco renders code and diffs, while xterm renders an interactive PTY instead of
pretending command-runner output is a terminal.

**First useful slice:** Monaco-backed file editor and diff viewer; Core APIs for
terminal open/list/stream/input/resize/close; Node Daemon PTY lifecycle scoped
to the validated workspace; xterm terminal tabs with attach, resize, input,
output, status and close handling. The bounded command runner remains separate
for traceable controlled checks.

**Current implementation note:** `0.1.7` adds shared protocol contracts for
workspace terminal commands and stream frames, Core routes all terminal traffic
through the node control channel and WebSocket client stream, Node owns PTY
creation and cleanup inside the workspace cwd, and Web uses Monaco plus xterm.js
as first-class renderers.

**Target direction:** Add durable replay endpoints, terminal output refs,
search/copy ergonomics, review decorations, selection/range actions, and richer
diff/review workflows without weakening the Core/Node authority boundary.

### 6. Daily-use hardening and deployment readiness

**Value:** The core workbench path is now useful enough that the next slice
should make it comfortable and reliable for everyday work, not only
feature-complete in isolated flows.

**Dependency:** The `0.1.7` workspace renderer/PTY baseline, controlled-dev
security baseline, current Core/Web dev profile, and host Node run path.

**First useful slice:** Rework the Web Control Panel around sustained use:
panel placement, information density, navigation rhythm, workspace/session
switching, terminal/editor/diff ergonomics, and empty/loading/error states.
Apply a visual design pass to make the current functionality feel coherent
enough for continuous use. Add a real server deployment path with documented
environment settings, reverse-proxy/TLS assumptions, persistent volumes, logs,
backup/restore expectations, and a CI/CD baseline that runs quality gates and
can deploy the controlled instance.

**Risk:** This slice can sprawl into redesigning future surfaces or pretending
the product is already a multi-user production release. Keep the scope tied to
the current single-user or controlled deployment and refine the detailed
checklist from actual daily use.

**Target direction:** Establish a stable personal/server operating mode that can
be used continuously while later trace, git/review, registry, plugin, artifact,
and task-runtime work is built.

### 6a. Provider-native persistent execution policy

**Value:** Makes persistent provider execution safe and understandable without
mistaking a workspace allow-list or Unix account for a provider sandbox.

**Dependency:** The 0.2.0 quality foundation and a provider-native persistent
runtime path that can pause for policy and approval decisions.

**First useful slice and exit criteria:** All four conditions are required:

1. sandboxed execution is the safe default;
2. unrestricted execution is available only through an explicit unsafe-mode
   switch;
3. provider approval requests are handled by a real Core/User/Node approval
   flow before execution continues;
4. the effective sandbox and approval policy is visible before start and in
   runtime trace/evidence.

**Accepted risk before delivery:** Audit finding P0-3 remains an accepted risk
for the controlled deployment. The 0.2.0 quality-foundation release does not
change the existing Codex launch flags, does not treat the current normalized
approval events as real enforcement, and does not claim team, cloud or
hostile-workload isolation.

**Target direction:** Apply the same explicit policy contract to later
provider-native persistent runtimes while preserving provider-specific
enforcement and evidence.

### 7. Causality and trace UX

**Value:** Reduces review cost by connecting result to evidence without dumping
raw logs into the user interface.

**First useful slice:** Coarse links from answers, commands, diffs, checks, and
artifacts to source events, with explicit unknown/missing-cause states and raw
fallbacks.

**Target direction:** Richer cause graph and trace timeline once event quality
and artifact semantics stabilize.

### 8. Git and review basics

**Value:** Developer work needs changed-file awareness and review ergonomics.

**First useful slice:** Branch/worktree snapshot, changed-file list, diff view,
check entry points, warning badges for risky workspace state.

**Target direction:** Git provider integration, PR/MR comment import, review
queues, CI follow-up loops, and review-ready task outputs.

### 9. Tool Registry v1

**Value:** Tools become system capabilities with permissions, routing, schemas,
UI contracts, and audit policy instead of hidden agent behavior.

**First useful slice:** Core-owned registry for Uprava-native workspace/session
tools and Node capabilities.

**Target direction:** External providers, MCP/native/hybrid adapters, tool call
trace, and agent-readable capability discovery.

### 10. Plugin Registry v1

**Value:** Uprava becomes extensible without hardcoding every tool, block, and
integration inside the workbench.

**First useful slice:** Installed plugin metadata, versions, configuration,
requested permissions, exposed tools, artifact types, and compatibility.

**Target direction:** Plugin-provided commands, renderers, link handlers,
workflow templates, and governed extension surfaces.

### 11. First external integrations

**Value:** Agent work must connect to real development systems without hiding
integration behavior behind text.

**First useful slice:** Git provider and Linear/task-tracker slices with visible
objects, actions, trace, and permission checks.

**Target direction:** Native, MCP, Node-local, external-provider, and hybrid
integration adapters.

### 12. Visual artifact system

**Value:** Results such as diffs, checks, timelines, reports, diagrams, and
dashboards should be inspectable UI objects, not only chat text.

**First useful slice:** First-class artifacts for diff/check reports and trace
timeline with source references and fallbacks.

**Target direction:** Artifact gallery, richer visual review, dashboards, UML,
forms, and embedded external views.

### 13. Dynamic UI from agents

**Value:** Agents and tools can return structured interactive surfaces where
text is the wrong shape.

**First useful slice:** Schema-driven or registered renderer blocks with
sanitized snapshots, source refs, permissions, and markdown/table fallback.

**Target direction:** Plugin-rendered blocks, controlled embeds, generated UI
sandboxing, and agent-readable UI state.

### 14. Task-based sandbox runtime

**Value:** Uprava can run bounded background work with explicit scope,
isolation, evidence, and review-ready output.

**First useful slice:** Task contract, isolated workspace/branch, context
package, event log, expected evidence, and result package.

**Target direction:** Durable workflow state, queues, CI/webhook wakeups, PR/MR
flow, and reproducible review packages.

### 15. Hybrid managed sessions

**Value:** Live sessions and background tasks become one work loop instead of
separate products.

**First useful slice:** A persistent session can spawn a bounded run and link
the run's evidence back into the session trace/review model.

**Target direction:** Orchestrated workflows, semi-deterministic pipelines,
handoff between live and bounded work, and review debt visibility.

### 16. Team/cloud model

**Value:** Uprava expands from personal workbench to shared distributed Agent OS.

**First useful slice:** Multi-user projects, roles, shared node visibility,
team audit trail, and managed Core deployment path.

**Target direction:** Managed cloud nodes, node pools, organization-level
plugin/integration governance, stronger secrets model, and billing if needed.

### 17. Beyond software development

**Value:** The same node, agent, tool, artifact, trace, and workflow model can
support broader knowledge work.

**First useful slice:** Pick one non-code vertical only after the developer
artifact/plugin model is strong enough to transfer.

**Target direction:** Research, analytics, documents, presentations, finance,
monitoring, and knowledge-base workflows.

### 18. Audit follow-up refactors

**Value:** Keeps the `0.1.6` audit fixes reviewable, contract-backed, and ready
for longer-running tools without mixing broad mechanical work into a behavior
hardening release.

**First useful slice:** Split Core command/event/session code and Node
state/command-runner code into focused modules under the current public
interfaces; add generated or schema-checked web protocol contracts; design an
async workspace command API for commands that outgrow bounded synchronous
execution.

**Target direction:** Make the command lifecycle, session projection, Node
state store, workspace command execution, and web protocol shapes independently
testable before Tool Registry and external integration work increase the
surface area.

## Open Queue Questions

- How strict must the first security baseline be before any non-local node is
  recommended?
- Which daily-use hardening items are mandatory before the first continuously
  used server deployment, and which can wait for later feature slices?
- Should git/review basics come before Tool Registry v1, or should registry
  contracts land first to avoid a hardcoded integration path?
- Which integration is the best first proof: GitHub/GitLab, Linear, MCP, or an
  internal Uprava-native tool set?
- How small can the first visual artifact system be while still changing the
  product experience beyond text?
