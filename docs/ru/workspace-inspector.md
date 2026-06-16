# Project Workspace Inspector

Статус: `draft`

## Короткое решение

Cortex должен включать **Project Workspace Inspector** в V01.

Это non-chat surface для просмотра проекта, в котором работает агент: project
tree, file viewer, lightweight text editor, workspace terminal sessions, command
and output history, diffs, checks and trace links.

Цель не в том, чтобы сначала построить полноценную browser IDE. Цель - сделать
agent work достаточно observable and interruptible, чтобы человек мог понять,
что происходит, внести небольшие прямые исправления и продолжить без просьбы к
агенту пересказывать свое окружение.

Позиция по editing and full-IDE sidecar описана в
[workspace-editing-and-ide-sidecar.md](workspace-editing-and-ide-sidecar.md).

## Product Role

Chat полезен для dialogue and intent. Для developer work этого недостаточно.

Пользователь также должен видеть:

- какой project and workspace использует агент;
- какие files есть в workspace;
- что содержит конкретный file or range;
- как внести небольшое прямое изменение, когда это быстрее, чем просить агента;
- какие terminal sessions открыты;
- какие commands выполнялись и что они вывели;
- какие files изменились;
- какие checks запускались;
- как files, commands, diffs, checks and artifacts связаны с trace.

Это превращает первый продукт из "agent chat with logs" в developer workbench.

## V01 Scope

Минимальный scope V01:

- project/workspace binding, видимый в UI;
- file tree rooted at the workspace;
- file metadata and file content view;
- lightweight text editing with explicit save/apply semantics;
- safe handling for large, binary, ignored, generated and permission-denied files;
- workspace-scoped terminal/PTY sessions with attach, detach, resize and close;
- command/output history tied to terminal sessions and agent events;
- basic diff view for workspace changes;
- basic check/test result entry points;
- addressable references to files, ranges, terminal sessions, commands, diffs
  and checks;
- trace links from agent events to workspace evidence.

Первая реализация может быть намеренно маленькой. File tree, file viewer/editor,
terminal panel and diff/check entry points уже проверяют product thesis.

## Explicit Non-Goals

V01 не должна пытаться стать полноценной IDE:

- нет требования full code editor;
- нет требования language server;
- нет требования rich refactoring, debugger or VS Code extension compatibility;
- нет arbitrary plugin-controlled workbench layout;
- нет direct client-to-node filesystem access;
- нет unrestricted shell access без Core permissions and Node-side enforcement.

Limited intervention actions являются частью направления, когда они снижают
стоимость review: save text edit, apply patch, open agent follow-up on a file
range or run a specific check. База должна оставаться inspect-first and
edit-light, not IDE-first.

## Architecture

Project Workspace Inspector - это UI surface, но authority распределена по
системе.

Core Backend отвечает за:

- project/workspace identity;
- user permissions and policy decisions;
- command routing to Node Daemon;
- event log and trace metadata;
- artifact and diff metadata;
- edit permission checks and edit event metadata;
- addressable references used by chat, trace, artifacts and review.

Node Daemon отвечает за:

- local workspace root resolution;
- path normalization and workspace boundary enforcement;
- file metadata, content reads and controlled text writes or patch applies;
- terminal/PTY lifecycle;
- process and command output streaming;
- local resource limits;
- local checks/tests execution;
- raw local logs when needed.

Web Control Panel отвечает за:

- file tree, viewer/editor buffer, tabs/panes, terminal panel, diff/check views
  and review ergonomics;
- user-initiated commands;
- navigation between chat, files, terminal, diff, checks, artifacts and trace;
- readable fallback states for unavailable files, missing terminals, denied
  permissions and disconnected nodes.

Core не должен напрямую читать filesystem каждой node. Clients не должны
подключаться напрямую к каждой node. Базовый путь:

```text
Web Control Panel -> Core Backend -> Node Daemon -> Workspace / PTY / Process
```

## Addressable Workspace References

Workspace objects должны быть referenceable, чтобы chat, agents, trace and UI
actions могли указывать на одну и ту же evidence.

Примеры references:

```text
workspace file
workspace file range
workspace edit
terminal session
terminal command
command output range
diff file
diff hunk
check run
check failure
artifact produced from workspace state
```

Эти references поддерживают:

- `@` mentions in chat;
- "ask agent about this file/range/hunk" actions;
- trace entries that point to concrete evidence;
- review decisions tied to changed files, user edits, commands and checks;
- plugin contributions for previews, actions and detail views.

Точная URI/schema пока открыта. Архитектурное правило: files, terminal sessions,
commands, diffs, checks and artifacts должны быть addressable objects, а не
только pixels in the UI.

## Security Boundaries

Workspace inspection нужно считать privileged access.

Важные boundaries:

- каждое file and terminal action scoped to a registered project/workspace;
- Node Daemon enforces local workspace boundary even if Core or client sends a
  malformed path;
- terminal creation and command execution require explicit permissions;
- file writes require explicit user action and permission checks;
- file reads must handle symlinks, ignored files, secrets, binary files and very
  large files intentionally;
- event and trace metadata should record who opened terminals, ran commands or
  invoked privileged workspace actions, including file writes;
- disconnected nodes should degrade to cached metadata and trace, not pretend
  the workspace is live.

## Feature Queue Directions

Feature queue может сделать surface extensible через Tool Registry and Plugin
Registry: file previews, extra actions, detail aspects and integration-aware
links.

Visual work surface items могут обогатить продукт terminal replay, structured
command history, richer editing/review flows, test reports, richer diff/review,
symbol/navigation aids and artifact galleries.

Task-based sandbox runtime может использовать те же workspace concepts для
review packages, isolated branches/workspaces, expected evidence and MR/PR
output.

Hybrid managed sessions могут позволить persistent session запускать bounded
task runs и связывать их workspace evidence обратно с той же review and trace
model.

## Открытые вопросы

- Должен ли V01 terminal быть fully interactive PTY, command runner или обоими?
- Должна ли V01 editing сохранять whole files, применять patches или
  поддерживать оба пути?
- Как должен работать file search: Node-local search first, indexed search или
  оба?
- Какая minimal reference schema нужна для path, range, command, output, diff,
  edit and check objects?
- Как различать session terminals and agent-owned terminals в UI and trace?
- Какие files должны быть hidden or redacted by default?
