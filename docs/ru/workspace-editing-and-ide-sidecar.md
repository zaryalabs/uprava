# Workspace Editing and IDE Sidecar

Статус: `draft`

## Короткое решение

Cortex должен поддерживать **basic workspace file editing**, не пытаясь стать
полноценной IDE в первой реализации.

V01 target:

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
Cortex.

Full browser IDE должна оставаться sidecar capability: Cortex может показывать
действие "open full IDE" на базе code-server, OpenVSCode Server, Theia или
другого provider, но основной Cortex workbench не должен зависеть от этой
архитектуры.

## Почему basic editing важен

Read-only inspection недостаточен для developer workbench.

Пользователь должен уметь:

- исправить небольшую typo or config value;
- изменить file перед тем, как попросить агента продолжить;
- напрямую редактировать docs, prompts, scripts or tests;
- сравнить manual changes with agent changes in a diff;
- использовать Cortex из browser, когда local editor access неудобен;
- сохранять traceability для human interventions, а не только agent actions.

Цель не "replace VS Code". Цель - "make the agent workspace operable".

## V01 Editing Scope

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

Editing остается routed through Cortex authority:

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
binary, generated or policy-denied, Cortex должен показать readable reason and
offer safe alternatives such as open externally or ask agent, when allowed.

## Component Strategy

UI должен использовать proven browser editor component за локальной abstraction.

Вероятные кандидаты:

- **CodeMirror 6** - lighter, modular, хорошо подходит для embedded text editing
  and custom workbench UI.
- **Monaco Editor** - ближе к VS Code behavior, силен для code viewing,
  diffing, models and line decorations, но heavier and more IDE-shaped.

Implementation choice должен оставаться open до появления web scaffold. Какой бы
component ни был выбран, Cortex должен wrapped it in local components such as
`FileViewer`, `FileEditor` and `DiffViewer`, чтобы остальной продукт не был
coupled to a specific editor library.

Terminal rendering также должен использовать proven browser terminal component,
скорее всего xterm.js, while Node Daemon owns the actual PTY/process lifecycle.

## IDE Sidecar

Cortex может показывать full IDE как sidecar, а не встраивать его в core
workbench.

Возможные sidecar providers:

- code-server;
- OpenVSCode Server;
- Theia-based IDE;
- managed devbox/cloud IDE provider.

Роль sidecar:

```text
Cortex remains system of record for projects, nodes, sessions, trace, artifacts,
review, permissions, and agent workflow.

Full IDE sidecar handles rich editing, LSP, extensions, refactoring, debugger,
and advanced developer ergonomics.
```

Это дает пользователям escape hatch для complex manual work, сохраняя основной
Cortex UI сфокусированным на agent supervision, workspace evidence,
traceability and review.

## Открытые вопросы

- Должна ли V01 editing сохранять whole files, применять patches или
  поддерживать оба пути?
- Должны ли edits быть allowed directly, require diff preview или depend on path
  risk?
- Какой editor component использовать первым: CodeMirror or Monaco?
- Сколько diff UI нужно до save versus after save?
- Как различать user edits and agent edits in trace and review?
- Когда должен появиться "open full IDE" sidecar: V01 experiment or feature
  queue item?
