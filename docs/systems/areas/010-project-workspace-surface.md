# A-010 Project Workspace Surface

Статус: `working-position`

Этот документ фиксирует рабочую позицию по ключевой механике `A-010 Project
Workspace Surface`.

## Vision

### Какую проблему решает механика

Uprava не должен быть только чатом агента. Для developer workflow пользователь
должен видеть и контролировать среду, в которой агент работает:

- дерево проекта;
- файлы и конкретные диапазоны;
- легкое редактирование текстовых файлов;
- терминальные сессии внутри workspace;
- историю команд и output;
- diff и check results;
- связь этих объектов с trace, событиями, agent turns and review.

Если этого нет, пользователь снова вынужден спрашивать агента "что там в
файлах?", "что ты запустил?", "где diff?", "можешь поправить мелочь?". Это
повышает стоимость review и делает человека зависимым от текстового пересказа
агента.

A-010 решает это как отдельную механику: **workspace должен быть видимой,
адресуемой и ограниченно редактируемой рабочей поверхностью**.

### Главная позиция

Первый workspace surface slice после V01 должен иметь не полноценную IDE, а
**inspect-first, edit-light project workspace surface**.

Формула:

```text
Project Workspace Surface
= file tree
+ file viewer/editor
+ terminal/PTY panel
+ command/output history
+ diff/check views
+ addressable workspace refs
+ trace/review links
+ optional full IDE sidecar later
```

Это ближе к легкому GitHub/GitLab web project view + terminal + basic editor,
чем к попытке встроить весь VS Code внутрь Uprava.

### Почему не полноценная IDE first

Полная IDE сразу потянет за собой:

- LSP;
- extension ecosystem;
- сложные editor services;
- refactoring;
- debugger;
- multi-root/multi-window semantics;
- сложное состояние workspace;
- отдельные security and trust models;
- риск, что Uprava станет оболочкой вокруг IDE, а не Agent OS.

Это слишком тяжелый центр тяжести для первого продукта.

Но полностью read-only модель тоже слабая. Пользователь должен иметь возможность
поправить маленькую вещь сам: doc/config/test/script/code fragment. Поэтому
workspace direction должен поддерживать базовое редактирование, но в узком
контракте, после read-only inspector foundation.

### First workspace surface target

Минимальная целевая цепочка:

```text
open project on node
-> open workspace surface
-> inspect file tree
-> open text file
-> make small edit in editor buffer
-> explicit save/apply
-> Node Daemon writes inside workspace boundary
-> Core records edit event, diff and trace reference
-> user or agent continues work
```

Терминал:

```text
open workspace terminal
-> Core checks permission
-> Node Daemon creates PTY inside workspace
-> Web renders xterm.js panel
-> input/output/resize stream through Core
-> command/output can be referenced from trace/review
```

### User-facing сценарии

1. **Inspect agent workspace**

   Пользователь открывает agent session и рядом видит project tree, opened file,
   terminal panel, diff/check state and trace. Ему не нужно просить агента
   пересказать файлы.

2. **Small manual edit**

   Пользователь открывает файл, меняет одну строку, сохраняет. Uprava показывает
   dirty state, сохраняет через Node Daemon, записывает событие и делает изменение
   видимым в diff/review.

3. **Terminal intervention**

   Пользователь открывает interactive workspace terminal for exploratory shell
   work, or runs a controlled check through the command runner. PTY output,
   status and exit frames stay attached to the workspace surface instead of
   being lost in a local terminal tab.

4. **Ask agent about selected evidence**

   Пользователь выделяет file range, diff hunk или terminal output range и
   вызывает action: `ask agent about this`, `continue from here`, `fix this`.
   Agent получает structured reference, а не расплывчатое текстовое описание.

5. **Open full IDE escape hatch**

   Если нужна полноценная ручная разработка, Uprava может показать action
   `Open full IDE`, который открывает code-server/OpenVSCode/Theia or another
   provider for the same workspace. Uprava остается system of record для session,
   trace, review and permissions.

### Agent-facing сценарии

Агент должен уметь ссылаться на те же объекты, которые видит пользователь:

```text
workspace file
workspace file range
workspace edit
terminal session
terminal command
terminal output range
diff file
diff hunk
check run
check failure
artifact
```

Это позволяет агенту:

- объяснять изменения через ссылки на файлы/diff/checks;
- отвечать на вопросы о выделенном диапазоне;
- продолжать работу после ручного edit пользователя;
- понимать, что пользователь уже вмешался в workspace;
- не прятать critical evidence в текстовом ответе.

### Scope boundaries

Первый полноценный workspace surface slice включает:

- file tree;
- file viewer;
- lightweight text editor;
- explicit save/apply;
- conflict detection на уровне "file changed since opened";
- terminal/PTY panel;
- command/output history;
- basic diff/check entry points;
- addressable workspace refs;
- audit/trace events for privileged actions.

Он не включает:

- полноценный IDE core;
- LSP как обязательное требование;
- VS Code extension compatibility;
- rich refactoring;
- debugger;
- collaborative editing;
- arbitrary plugin-controlled workbench layout;
- direct client-to-node filesystem access outside Core routing.

## Architecture

### Relationship with other mechanics

A-010 находится на стыке нескольких механик:

- `A-001 Distributed Architecture`: определяет Core / Node Daemon / client
  boundary. A-010 использует эту границу для files, PTY and processes.
- `A-002 Run Mode`: session/run связывается с project/workspace. A-010 делает
  этот workspace видимым и управляемым.
- `A-003 Distributed Runtime Coordination`: dispatch, event ordering, stale node,
  workspace availability and resource warnings нужны для file/terminal commands.
- `A-004 Modular UI and Work Surface`: A-010 является concrete workbench surface
  with known extension points, not arbitrary plugin UI.
- `A-006 Visual Rendering and Artifact Semantics`: diff/check/terminal/file views
  должны иметь source-of-truth, fallback, refs and artifact promotion rules.
- `A-008 Go to Source and Causality UX`: file ranges, terminal commands, diff
  hunks and checks становятся evidence/cause refs.

### Core model

Core не должен читать файловую систему ноды напрямую. Его роль:

- хранить Project/Placement binding;
- проверять permissions;
- маршрутизировать file/terminal/edit commands к нужной Node;
- хранить event metadata;
- хранить placement-scoped workspace refs без второй Workspace identity;
- связывать edits/commands/diffs/checks с trace/review;
- знать, какой sidecar IDE provider доступен, если он включен.

Минимальные сущности:

```text
ProjectPlacement
PlacementRef
FileRef
FileRangeRef
WorkspaceEditRef
TerminalSessionRef
TerminalCommandRef
TerminalOutputRangeRef
DiffRef
DiffHunkRef
CheckRunRef
```

### Node Daemon responsibilities

Node Daemon является authority рядом с workspace:

- resolve workspace root;
- normalize paths;
- enforce workspace boundary;
- handle symlinks intentionally;
- read file metadata/content;
- perform controlled text writes or patch applies;
- create/attach/detach/resize/close PTY sessions;
- stream stdout/stderr/terminal data;
- run checks/tests where allowed;
- report file/terminal/check errors as structured events.

Node Daemon не должен доверять path, пришедшему от клиента или Core. Даже если
Core ошибся, Node обязан не выпустить action за пределы разрешенного workspace.

### Web Control Panel responsibilities

Web отвечает за UX, но не за authority:

- file tree;
- viewer/editor buffer;
- dirty state;
- save/apply flow;
- terminal panel;
- diff/check views;
- selected range and context actions;
- fallback states;
- links to trace/review/chat.

Web не решает самостоятельно, можно ли читать/писать файл или открыть shell. Он
инициирует command, а Core/Node проверяют права и границы.

### File editing lifecycle

Базовый lifecycle:

```text
OpenFile(path)
-> Core checks read permission
-> Node reads metadata/content/version
-> Web opens editor buffer
-> User edits buffer
-> SaveFile(path, expected_version, content or patch)
-> Core checks write permission
-> Node checks path boundary and file version
-> Node writes atomically or applies patch
-> Node returns new version/status
-> Core records WorkspaceEdit event
-> Diff/review state updates
```

Open question: первый editing slice должен сохранять whole file, применять patch
or support both. Whole file проще. Patch лучше для trace and conflict handling.
Практичная позиция: начать с whole-file save для маленьких text files, но Core/Node command
shape проектировать так, чтобы patch apply появился без переписывания модели.

### Terminal lifecycle

Базовый lifecycle:

```text
CreateTerminal(placement_id, shell/profile)
-> Core checks terminal permission
-> Node creates PTY in workspace cwd
-> Web attaches to terminal stream
-> User input/output/resize flow through Core
-> Terminal events are stored as compact metadata
-> Important output ranges can become addressable refs
-> Close/detach preserves explicit terminal status
```

Решенная развилка: полноценный interactive PTY and command runner both exist,
but they have different jobs:

- interactive PTY нужен для привычной работы;
- command runner проще трассировать и безопаснее для controlled checks.

Implementation line explicit: xterm renders an interactive PTY lifecycle, not
command-runner output. Controlled checks such as `make l` and `make c` continue
through the bounded command runner.

### Addressability

Workspace surface должна быть адресуемой.

Примеры refs:

```text
uprava://workspace/{placement_id}/file/{path}
uprava://workspace/{placement_id}/file/{path}#L10-L20
uprava://workspace/{placement_id}/edit/{edit_id}
uprava://workspace/{placement_id}/terminal/{terminal_id}
uprava://workspace/{placement_id}/terminal/{terminal_id}/command/{command_id}
uprava://workspace/{placement_id}/diff/{diff_id}/hunk/{hunk_id}
uprava://workspace/{placement_id}/check/{check_run_id}/failure/{failure_id}
```

Это не финальный URI contract, а direction: UI, trace, artifacts, review and
agent prompts должны ссылаться на одни и те же workspace objects.
В 0.2.0 `{placement_id}` является identity физического Placement; `Workspace`
называет user-facing surface и не создаёт отдельную persisted entity или ref.

### Permissions and trust

Отдельные permissions:

```text
workspace.read_tree
workspace.read_file
workspace.write_file
workspace.open_terminal
workspace.write_terminal
workspace.run_check
workspace.open_ide_sidecar
```

Policy может зависеть от:

- user;
- project;
- node;
- workspace;
- path pattern;
- file type;
- generated/ignored status;
- session/run mode;
- local vs remote deployment.

### Failure modes

| Failure | UX / system behavior |
| --- | --- |
| Node offline | Surface показывает cached metadata/trace where possible, live file/terminal actions disabled. |
| Workspace missing | Hard block live actions, keep historical session readable. |
| Permission denied | Show explicit denied state, offer safe alternatives if available. |
| File too large/binary | Do not open editor; show metadata and allowed actions. |
| File changed after open | Conflict UI before save/apply. |
| Symlink escapes workspace | Node rejects action, event records rejection reason. |
| Terminal exits | Keep terminal status/output history readable if retention policy allows. |
| Sidecar IDE unavailable | Hide or disable `Open full IDE`, do not block Uprava surface. |

### Component strategy

Implementation should use proven components, but hide them behind Uprava-owned
UI boundaries:

```text
FileTree
FileViewer
FileEditor
DiffViewer
TerminalPanel
CheckPanel
PlacementWorkspaceLink
```

Chosen first-party renderers:

- Monaco for file viewer/editor/diff;
- xterm.js for interactive terminal rendering;
- code-server/OpenVSCode/Theia as optional sidecar providers later.

Library choice is not the key mechanic. The key mechanic is the contract:
workspace objects are visible, editable in narrow scope, addressable, routed
through Core, enforced by Node, and connected to trace/review.

### Open questions

- Should the first editing slice save whole file, apply patches, or support both?
- Which files are editable by default: all text files, docs/config only, or
  policy-based?
- How much diff preview is required before save?
- How do we distinguish human edits, agent edits and external git changes in UI?
- How long do terminal output buffers live, and where are they stored?
- Should terminal session persistence use PTY lifecycle only, shell history, or
  tmux-like persistence later?
- When does full IDE sidecar appear: workspace-surface experiment or later feature queue item?
