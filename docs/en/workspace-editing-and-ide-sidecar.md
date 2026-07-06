# Workspace Editing and IDE Sidecar

Status: `draft`

## Short Decision

Uprava should support **basic workspace file editing** without trying to become a
full IDE in the first workspace editing implementation.

The first editing slice target is:

```text
file tree
-> open text file
-> edit buffer
-> explicit save/apply through Core
-> Node Daemon writes inside workspace boundary
-> Core records event, diff, and trace reference
```

This gives the user a practical way to intervene in agent work, fix small
issues, adjust docs/config/code, and review the result without leaving Uprava.

A full browser IDE should remain a sidecar capability: Uprava may expose an
"open full IDE" action backed by code-server, OpenVSCode Server, Theia, or
another provider, but the main Uprava workbench should not depend on that
architecture.

## Why Basic Editing Matters

Read-only inspection is not enough for a developer workbench.

The user should be able to:

- fix a small typo or config value;
- adjust a file before asking the agent to continue;
- edit docs, prompts, scripts, or tests directly;
- compare manual changes with agent changes in a diff;
- use Uprava from a browser when local editor access is inconvenient;
- preserve traceability for human interventions, not only agent actions.

The goal is not "replace VS Code". The goal is "make the agent workspace
operable".

## First Editing Scope

Minimal useful scope:

- text-file editing for files inside a registered workspace;
- one opened file per editor instance, with tabs/panes optional;
- line numbers, syntax highlighting, search, and basic keyboard editing;
- explicit dirty state and explicit save/apply;
- conflict detection when file content changed after the editor opened;
- safe fallback for large, binary, generated, ignored, missing, or denied files;
- diff preview before or after save when practical;
- event log entry for user-authored file writes;
- trace/reference link from edit event to file path and optional line range.

This scope is intentionally smaller than IDE editing. It does not require
language servers, refactoring, inline diagnostics, project-wide rename, debugger
integration, or VS Code extension compatibility.

## Architecture Contract

Editing remains routed through Uprava authority:

```text
Web editor buffer
-> Core permission check and command routing
-> Node Daemon workspace boundary enforcement
-> atomic text write or patch apply
-> event/diff/trace update
```

Core owns:

- edit permission checks;
- routing to the correct node/workspace;
- edit event metadata;
- diff/check/artifact linkage;
- audit and trace references.

Node Daemon owns:

- path normalization;
- symlink and workspace boundary enforcement;
- file stat/version checks;
- read and write operations;
- atomic write or patch apply;
- filesystem error reporting.

Web Control Panel owns:

- editor buffer state;
- dirty state;
- save/apply UX;
- conflict UI;
- diff/review presentation;
- navigation between file, diff, trace, terminal, and chat.

## Editing Safety Rules

Basic editing is still privileged filesystem access.

Rules:

- never write outside the registered workspace root;
- treat symlinks, hidden files, generated files, and ignored files deliberately;
- require explicit user action to save/apply;
- detect stale editor buffers before overwriting changed files;
- keep a compact event trail for human edits;
- make changed files visible in diff/review;
- allow policies to disable editing per project, node, user, file type, or path.

The first version can choose conservative defaults. If a file is too large,
binary, generated, or policy-denied, Uprava should show a readable reason and
offer safe alternatives such as open externally or ask agent, when allowed.

## Component Strategy

The UI should use a proven browser editor component behind a local abstraction.

Likely candidates:

- **CodeMirror 6** - lighter, modular, strong fit for embedded text editing and
  custom workbench UI.
- **Monaco Editor** - closer to VS Code behavior, strong for code viewing,
  diffing, models, and line decorations, but heavier and more IDE-shaped.

The implementation choice should stay open until the web scaffold exists.
Whichever component is chosen, Uprava should wrap it in local components such as
`FileViewer`, `FileEditor`, and `DiffViewer` so the rest of the product is not
coupled to a specific editor library.

Terminal rendering should likewise use a proven browser terminal component, most
likely xterm.js, while Node Daemon owns the actual PTY/process lifecycle.

## IDE Sidecar

Uprava can expose a full IDE as a sidecar rather than embedding it into the core
workbench.

Possible sidecar providers:

- code-server;
- OpenVSCode Server;
- Theia-based IDE;
- managed devbox/cloud IDE provider.

Sidecar role:

```text
Uprava remains system of record for projects, nodes, sessions, trace, artifacts,
review, permissions, and agent workflow.

Full IDE sidecar handles rich editing, LSP, extensions, refactoring, debugger,
and advanced developer ergonomics.
```

This gives users an escape hatch for complex manual work while keeping Uprava's
main UI focused on agent supervision, workspace evidence, traceability, and
review.

## Open Questions

- Should the first editing slice save whole files, apply patches, or support both?
- Should edits be allowed directly, require diff preview, or depend on path risk?
- Which editor component should be used first: CodeMirror or Monaco?
- How much diff UI is needed before saving versus after saving?
- How should user edits and agent edits be distinguished in trace and review?
- When should the "open full IDE" sidecar appear: workspace-surface experiment
  or later feature queue item?
