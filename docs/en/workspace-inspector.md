# Project Workspace Inspector

Status: `active`

## Short Decision

Uprava should add a **Project Workspace Inspector** after V01, as the first major
workbench expansion once the distributed agent control panel is usable.

This is the non-chat surface for looking into the project where an agent works:
project tree, file viewer, lightweight text editor, workspace terminal sessions,
command and output history, diffs, checks, and trace links.

The goal is not to build a full browser IDE first. The goal is to make agent work
observable and interruptible enough that a human can understand what is
happening, make small direct corrections, and continue without asking the agent
to summarize its own environment.

The editing and full-IDE sidecar position is detailed in
[workspace-editing-and-ide-sidecar.md](workspace-editing-and-ide-sidecar.md).

## V01 Reference Inspector Slot

V01 reserves the right-side inspector stack before the full Project Workspace
Inspector ships. This first slot is reference-oriented, not a file browser or
terminal surface.

The Web Control Panel can open and copy `UpravaRef` objects from visible session
timeline blocks, session evidence-projection entries, nodes, placements,
sessions, runtimes, events, commands, approvals and warnings. The panel resolves
details from the currently loaded Core snapshots and session event log. Future
workspace, terminal, diff, check, tool-call and external refs render explicit
unavailable or not-implemented states instead of broken links or invented
targets.

This keeps the V01 workbench traceable while preserving the later inspector
boundary: direct workspace inspection still goes through Core and Node Daemon
capabilities when that feature queue slice ships.

## Product Role

Chat is useful for dialogue and intent. It is not enough for developer work.

The user also needs to see:

- what project and workspace the agent is using;
- which files exist in the workspace;
- what a specific file or range contains;
- how to make a small direct edit when that is faster than asking the agent;
- what terminal sessions are open;
- what commands ran and what they produced;
- which files changed;
- which checks were run;
- how files, commands, diffs, checks, and artifacts connect to the trace.

This turns the V01 control panel from "distributed agent chat with lifecycle
state" into a developer workbench.

## First Useful Slice

The first useful inspector slice:

- project/workspace binding visible in the UI;
- file tree rooted at the workspace;
- file metadata and file content view;
- safe handling for large, binary, ignored, generated, and permission-denied files;
- addressable references to files and ranges;
- trace/event links from agent sessions to known workspace evidence where
  available.

The first implementation can be intentionally small and read-only. A file tree
and safe text viewer already validate the next product step without pulling
editor, terminal, diff, checks, tools, or plugins into V01.

The implemented read-only slice uses the existing Core-to-Node command channel:
Core authenticates the Web request, records a placement-scoped workspace command,
dispatches it to the Node Daemon, waits briefly for a typed command result, and
returns the tree or file payload to the Web Control Panel. The Node Daemon owns
path normalization and local filesystem access, including workspace boundary
checks, allowed-root checks, symlink stop-points, text-size caps, binary/large
classification, generated/ignored classification, and permission-denied states.

The file tree uses Headless Tree for accessible expansion, focus, and keyboard
navigation. Directory contents load lazily through Core one level at a time, so
browsing is not limited by a fixed workspace-tree depth. Node returns
directories before files and caps each directory response at the first 100
sorted entries, with explicit truncation metadata. Dotfiles and dot-directories
remain visible. Generated and ignored paths are visually classified but may
still be expanded explicitly; workspace-boundary and symlink stop-point
enforcement remains unchanged.

The implemented intervention layer adds:

- Monaco-backed lightweight text editing with explicit save/apply semantics;
- workspace-scoped no-shell command runner with bounded timeout, bounded output
  and an explicit controlled-dev executable allow-list: `cargo`, `git`, `make`,
  `node`, `npm`, `pnpm`, `bun` and `rustc`;
- persisted command/output history tied to placement-scoped commands;
- Monaco-backed diff view for workspace changes;
- basic check/test result entry points;
- addressable references to terminal sessions, commands, output ranges, diffs,
  checks, and edits.

The implemented renderer and PTY slice adds:

- first-class Monaco renderers for file editing and diff viewing;
- first-class xterm.js terminal tabs for interactive workspace PTY sessions;
- Core terminal APIs for open, list, attach/stream, resize, input and close;
- Node-owned PTY lifecycle scoped to the validated workspace cwd, with
  shell-profile policy, resize handling, close/cleanup, status/exit frames and
  bounded replay.

The command runner remains separate from xterm. It is still the right path for
traceable, bounded and policy-controlled checks such as `make l` and `make c`.

## Explicit Non-Goals

The first inspector and intervention slices should not try to become a full IDE:

- no full IDE requirement;
- no language server requirement;
- no rich refactoring, debugger, or VS Code extension compatibility requirement;
- no arbitrary plugin-controlled workbench layout before Plugin Registry exists;
- no direct client-to-node filesystem access;
- no unrestricted shell access without Core permissions and Node-side enforcement.

Limited intervention actions are part of the direction when they reduce review
cost: saving a text edit, applying a patch, opening an agent follow-up on a file
range, or running a specific check. The base should remain inspect-first and
edit-light, not IDE-first.

## Architecture

The Project Workspace Inspector is a UI surface, but its authority is split
across the system.

Core Backend owns:

- project/workspace identity;
- user permissions and policy decisions;
- command routing to Node Daemon;
- event log and trace metadata;
- artifact and diff metadata;
- edit permission checks and edit event metadata;
- addressable references used by chat, trace, artifacts, and review.

Node Daemon owns:

- local workspace root resolution;
- path normalization and workspace boundary enforcement;
- file metadata, content reads, and controlled text writes or patch applies;
- terminal/PTY lifecycle;
- process and command output streaming;
- local resource limits;
- local checks/tests execution;
- raw local logs when needed.

Web Control Panel owns:

- file tree, viewer/editor buffer, tabs/panes, terminal panel, diff/check views, and review ergonomics;
- user-initiated commands;
- navigation between chat, files, terminal, diff, checks, artifacts, and trace;
- readable fallback states for unavailable files, missing terminals, denied
  permissions, and disconnected nodes.

Core should not read every node filesystem directly. Clients should not connect
directly to every node. The base path is:

```text
Web Control Panel -> Core Backend -> Node Daemon -> Workspace / PTY / Process
```

## Addressable Workspace References

Workspace objects should be referenceable so chat, agents, trace, and UI actions
can point at the same evidence.

Reference examples:

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

These references support:

- `@` mentions in chat;
- "ask agent about this file/range/hunk" actions;
- trace entries that point to concrete evidence;
- review decisions tied to changed files, user edits, commands, and checks;
- plugin contributions for previews, actions, and detail views.

The exact URI/schema is still open. The architectural rule is that files,
terminal sessions, commands, diffs, checks, and artifacts must be addressable
objects, not only pixels in the UI.

## Security Boundaries

Workspace inspection must be treated as privileged access.

Important boundaries:

- every file and terminal action is scoped to a registered project/workspace;
- Node Daemon enforces the local workspace boundary even if Core or client sends
  a malformed path;
- terminal creation and command execution require explicit permissions;
- file writes require explicit user action and permission checks;
- file reads must handle symlinks, ignored files, secrets, binary files, and very
  large files intentionally;
- event and trace metadata should record who opened terminals, ran commands, or
  invoked privileged workspace actions, including file writes;
- disconnected nodes should degrade to cached metadata and trace, not pretend the
  workspace is live.

## Feature Queue Directions

The feature queue can make the surface extensible through the Tool Registry and
Plugin Registry: file previews, extra actions, detail aspects, and
integration-aware links.

Visual work surface items can enrich the product with terminal replay,
structured command history, richer editing/review flows, test reports, richer
diff/review, symbol/navigation aids, and artifact galleries.

Task-based sandbox runtime can reuse the same workspace concepts for review
packages, isolated branches/workspaces, expected evidence, and MR/PR output.

Hybrid managed sessions can let a persistent session spawn bounded task runs and
connect their workspace evidence back into the same review and trace model.

## Open Questions

- Should the first intervention editor save whole files, apply patches, or
  support both?
- How should file search work: Node-local search first, indexed search, or both?
- What is the minimal reference schema for path, range, command, output, diff,
  edit, and check objects?
- How should session terminals, user-owned workspace terminals and agent-owned
  terminals be distinguished in UI and trace?
- Which files should be hidden or redacted by default?
