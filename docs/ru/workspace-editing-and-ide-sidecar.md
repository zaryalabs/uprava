# Workspace Editing and IDE Sidecar

Статус: `draft`

## Короткое решение

Uprava должен поддерживать **basic workspace file editing**, не пытаясь стать
полноценной IDE в первой workspace editing implementation.

First editing slice target:

```text
file tree
-> open text file
-> edit buffer
-> explicit save/apply through Core
-> Node Daemon writes inside workspace boundary
-> Core records event, diff, and trace reference
```

Это дает пользователю практический способ вмешиваться в agent work, исправлять
небольшие issues, менять docs/config/code and review the result без выхода из
Uprava.

Full browser IDE должна оставаться sidecar capability: Uprava может показывать
действие "open full IDE" на базе code-server, OpenVSCode Server, Theia или
другого provider, но основной Uprava workbench не должен зависеть от этой
архитектуры.

## Почему basic editing важен

Read-only inspection недостаточен для developer workbench.

Пользователь должен уметь:

- исправить небольшую typo or config value;
- изменить file перед тем, как попросить агента продолжить;
- напрямую редактировать docs, prompts, scripts or tests;
- сравнить manual changes with agent changes in a diff;
- использовать Uprava из browser, когда local editor access неудобен;
- сохранять traceability для human interventions, а не только agent actions.

Цель не "replace VS Code". Цель - "make the agent workspace operable".

## First Editing Scope

Минимальный полезный scope:

- text-file editing for files inside a registered workspace;
- one opened file per editor instance, with tabs/panes optional;
- line numbers, syntax highlighting, search and basic keyboard editing;
- explicit dirty state and explicit save/apply;
- conflict detection when file content changed after the editor opened;
- safe fallback for large, binary, generated, ignored, missing or denied files;
- diff preview before or after save when practical;
- event log entry for user-authored file writes;
- trace/reference link from edit event to file path and optional line range.

Этот scope намеренно меньше IDE editing. Он не требует language servers,
refactoring, inline diagnostics, project-wide rename, debugger integration or
VS Code extension compatibility.

## Architecture Contract

Editing остается routed through Uprava authority:

```text
Web editor buffer
-> Core permission check and command routing
-> Node Daemon workspace boundary enforcement
-> atomic text write or patch apply
-> event/diff/trace update
```

Core отвечает за:

- edit permission checks;
- routing to the correct node/workspace;
- edit event metadata;
- diff/check/artifact linkage;
- audit and trace references.

Node Daemon отвечает за:

- path normalization;
- symlink and workspace boundary enforcement;
- file stat/version checks;
- read and write operations;
- atomic write or patch apply;
- filesystem error reporting.

Web Control Panel отвечает за:

- editor buffer state;
- dirty state;
- save/apply UX;
- conflict UI;
- diff/review presentation;
- navigation between file, diff, trace, terminal and chat.

## Editing Safety Rules

Basic editing все равно является privileged filesystem access.

Правила:

- never write outside the registered workspace root;
- treat symlinks, hidden files, generated files and ignored files deliberately;
- require explicit user action to save/apply;
- detect stale editor buffers before overwriting changed files;
- keep a compact event trail for human edits;
- make changed files visible in diff/review;
- allow policies to disable editing per project, node, user, file type or path.

Первая версия может выбрать conservative defaults. Если file is too large,
binary, generated or policy-denied, Uprava должен показать readable reason and
offer safe alternatives such as open externally or ask agent, when allowed.

## Component Strategy

Web Control Panel теперь использует proven browser components за локальными
abstractions:

- **Monaco Editor** powers `FileEditor` and `DiffViewer`, давая workspace
  surface code-oriented editing, models, syntax highlighting, diff rendering and
  future room for selections, range actions and review decorations.
- **xterm.js** powers interactive workspace terminal rendering. Это только
  browser terminal renderer: Core routes the stream, а Node Daemon owns the
  actual PTY/process lifecycle.

Uprava должен продолжать скрывать эти libraries за local components such as
`FileViewer`, `FileEditor`, `DiffViewer`, `TerminalTabs` and
`XtermTerminalPanel`, чтобы остальной продукт не был coupled to editor or
terminal library APIs.

Command runner остается отдельной controlled surface для bounded,
policy-controlled and traceable checks. Его не нужно collapse into xterm,
потому что xterm представляет interactive PTY lifecycle, а не structured command
execution record.

## IDE Sidecar

Uprava может показывать full IDE как sidecar, а не встраивать его в core
workbench.

Возможные sidecar providers:

- code-server;
- OpenVSCode Server;
- Theia-based IDE;
- managed devbox/cloud IDE provider.

Роль sidecar:

```text
Uprava remains system of record for projects, nodes, sessions, trace, artifacts,
review, permissions, and agent workflow.

Full IDE sidecar handles rich editing, LSP, extensions, refactoring, debugger,
and advanced developer ergonomics.
```

Это дает пользователям escape hatch для complex manual work, сохраняя основной
Uprava UI сфокусированным на agent supervision, workspace evidence,
traceability and review.

## Открытые вопросы

- Должен ли первый editing slice сохранять whole files, применять patches или
  поддерживать оба пути?
- Должны ли edits быть allowed directly, require diff preview или depend on path
  risk?
- Сколько diff UI нужно до save versus after save?
- Как различать user edits and agent edits in trace and review?
- Когда должен появиться "open full IDE" sidecar: workspace-surface experiment
  or later feature queue item?
