# Cortex Feature Queue

Status: `draft`

This document uses an implementation queue instead of a phase-based roadmap.

The queue is not a calendar, milestone ladder, or delivery promise. It is a
ranked set of product and architecture slices ordered by dependency, complexity,
risk, and value. Items can move as the design sharpens.

## Queue Rules

Each queue item should capture:

- **Value** - why this matters to the user or to Cortex as a system.
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

| Order | Mechanism / Feature Slice | First Useful Slice | Dependency | Complexity |
| --- | --- | --- | --- | --- |
| 0 | V01 Developer Node Workbench | First usable product cut | Current design baseline | High |
| 1 | Runtime/session hardening | Robust lifecycle, resume, stop, blocked, stale states | V01 runtime path | Medium |
| 2 | Workspace reference model | Stable refs for files, ranges, commands, diffs, checks, artifacts, and trace | V01 workspace surface | Medium |
| 3 | Causality and trace UX | Coarse source/cause links with raw fallback | Workspace refs, event log | Medium |
| 4 | Git and review basics | Better diff, branch/worktree awareness, check results | Workspace surface, trace | Medium |
| 5 | Tool Registry v1 | Real tool metadata, permissions, routing, and audit policy | V01 internal registry shape | High |
| 6 | Plugin Registry v1 | Installed plugin metadata, configuration, exposed tools, and artifact types | Tool Registry v1 | High |
| 7 | First external integrations | Git provider and task tracker integration slices | Tool/Plugin Registry | High |
| 8 | Visual artifact system | Test reports, richer diffs, timelines, dashboards/forms as first-class artifacts | Trace, registry contracts | High |
| 9 | Dynamic UI from agents | Schema/tool/plugin-rendered UI with safe fallbacks | Visual artifact system, plugins | High |
| 10 | Task-based sandbox runtime | Bounded run contract, isolated workspace, expected evidence | Runtime, workspace, trace | Very high |
| 11 | Hybrid managed sessions | Persistent session can spawn bounded runs and merge evidence back | Task runtime | Very high |
| 12 | Team/cloud model | Users, roles, shared projects, managed Core/nodes | Mature personal workflow | Very high |
| 13 | Beyond software development | Research, analytics, documents, finance, knowledge workflows | Mature artifact/plugin model | Very high |

## Queue Details

### 0. V01 Developer Node Workbench

**Value:** Gives the first tactile product: a user can run Core, connect a
node, start a persistent Codex-backed session, inspect workspace state, use a
terminal, edit a file, and review trace/diff.

**First useful slice:** Defined in [v01.md](v01.md).

**Target direction:** Keep the first product small while preserving the system
model for providers, tools, plugins, visual artifacts, task runs, mobile, and
team/cloud modes.

### 1. Runtime/session hardening

**Value:** Makes live agent work feel reliable instead of like a wrapped CLI.

**First useful slice:** Clear lifecycle states, explicit expiry/resume behavior,
blocked approvals, interrupt/stop semantics, stale node handling, and degraded
resume messaging.

**Target direction:** Support multiple runtime strategies and provider adapters
without changing Core/UI concepts.

### 2. Workspace reference model

**Value:** Lets chat, trace, artifacts, review, and agents point at the same
workspace evidence.

**First useful slice:** Stable references for file, file range, edit, terminal
session, command, output range, diff hunk, check result, artifact, turn, and
trace event.

**Target direction:** Shared addressability for UI navigation, agent prompts,
review decisions, plugin blocks, and task-run packages.

### 3. Causality and trace UX

**Value:** Reduces review cost by connecting result to evidence without dumping
raw logs into the user interface.

**First useful slice:** Coarse links from answers, commands, diffs, checks, and
artifacts to source events, with explicit unknown/missing-cause states and raw
fallbacks.

**Target direction:** Richer cause graph and trace timeline once event quality
and artifact semantics stabilize.

### 4. Git and review basics

**Value:** Developer work needs changed-file awareness and review ergonomics.

**First useful slice:** Branch/worktree snapshot, changed-file list, diff view,
check entry points, warning badges for risky workspace state.

**Target direction:** Git provider integration, PR/MR comment import, review
queues, CI follow-up loops, and review-ready task outputs.

### 5. Tool Registry v1

**Value:** Tools become system capabilities with permissions, routing, schemas,
UI contracts, and audit policy instead of hidden agent behavior.

**First useful slice:** Core-owned registry for Cortex-native workspace/session
tools and Node capabilities.

**Target direction:** External providers, MCP/native/hybrid adapters, tool call
trace, and agent-readable capability discovery.

### 6. Plugin Registry v1

**Value:** Cortex becomes extensible without hardcoding every tool, block, and
integration into the workbench.

**First useful slice:** Installed plugin metadata, versions, configuration,
requested permissions, exposed tools, artifact types, and compatibility.

**Target direction:** Plugin-provided commands, renderers, link handlers,
workflow templates, and governed extension surfaces.

### 7. First external integrations

**Value:** Agent work must connect to real development systems without hiding
integration behavior behind text.

**First useful slice:** Git provider and Linear/task-tracker slices with visible
objects, actions, trace, and permission checks.

**Target direction:** Native, MCP, Node-local, external-provider, and hybrid
integration adapters.

### 8. Visual artifact system

**Value:** Results such as diffs, checks, timelines, reports, diagrams, and
dashboards should be inspectable UI objects, not only chat text.

**First useful slice:** First-class artifacts for diff/check reports and trace
timeline with source references and fallbacks.

**Target direction:** Artifact gallery, richer visual review, dashboards, UML,
forms, and embedded external views.

### 9. Dynamic UI from agents

**Value:** Agents and tools can return structured interactive surfaces where
text is the wrong shape.

**First useful slice:** Schema-driven or registered renderer blocks with
sanitized snapshots, source refs, permissions, and markdown/table fallback.

**Target direction:** Plugin-rendered blocks, controlled embeds, generated UI
sandboxing, and agent-readable UI state.

### 10. Task-based sandbox runtime

**Value:** Cortex can run bounded background work with explicit scope,
isolation, evidence, and review-ready output.

**First useful slice:** Task contract, isolated workspace/branch, context
package, event log, expected evidence, and result package.

**Target direction:** Durable workflow state, queues, CI/webhook wakeups, PR/MR
flow, and reproducible review packages.

### 11. Hybrid managed sessions

**Value:** Live sessions and background tasks become one work loop instead of
separate products.

**First useful slice:** A persistent session can spawn a bounded run and link
the run's evidence back into the session trace/review model.

**Target direction:** Orchestrated workflows, semi-deterministic pipelines,
handoff between live and bounded work, and review debt visibility.

### 12. Team/cloud model

**Value:** Cortex expands from personal workbench to shared distributed Agent OS.

**First useful slice:** Multi-user projects, roles, shared node visibility,
team audit trail, and managed Core deployment path.

**Target direction:** Managed cloud nodes, node pools, organization-level
plugin/integration governance, stronger secrets model, and billing if needed.

### 13. Beyond software development

**Value:** The same node, agent, tool, artifact, trace, and workflow model can
support broader knowledge work.

**First useful slice:** Pick one non-code vertical only after the developer
artifact/plugin model is strong enough to transfer.

**Target direction:** Research, analytics, documents, presentations, finance,
monitoring, and knowledge-base workflows.

## Open Queue Questions

- Which queue item should be the first post-V01 product hardening slice?
- Should git/review basics come before Tool Registry v1, or should registry
  contracts land first to avoid a hardcoded integration path?
- Which integration is the best first proof: GitHub/GitLab, Linear, MCP, or an
  internal Cortex-native tool set?
- How small can the first visual artifact system be while still changing the
  review experience?
- Which task-based runtime slice is useful before full durable workflow state?
