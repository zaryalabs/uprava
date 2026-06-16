# Cortex Product Evolution

Status: `draft`

This document replaces the old stage-based model. The canonical description of
the first version now lives in [`v01.md`](v01.md), and the order of follow-up
work lives in [`feature-queue.md`](feature-queue.md).

## Working Model

- `V01` is the first tangible product version: what a user can start, open,
  see, touch, and verify.
- `Feature Queue` is an implementation queue ordered by dependency, complexity,
  risk, and product value.
- `Feature Inventory` is the full inventory of known ideas and directions
  without a promised implementation order.
- Design docs describe the target model of key mechanisms without rigid
  time-based slicing.

## V01

The first product version is **Developer Node Workbench**:

- Core Backend and Web Control Panel;
- one or more nodes with Node Daemon;
- persistent Codex-backed session through Agent Provider Adapter;
- project/workspace binding;
- chat/session view;
- Project Workspace Inspector: file tree, file viewer, lightweight text editor,
  workspace terminal/PTY sessions, command/output history, and basic diff/check
  entry points;
- basic trace and event log;
- minimal Tool Registry, Plugin Registry, and visual block/artifact contract
  shape.

V01 validates the thesis:

```text
Persistent Runtime + Node Daemon + Core UI
give more control, continuity, and reviewability
than a regular local agent chat.
```

## Next

Further development is not fixed as a linear chain of stages. Instead, the next
slices should be selected from the feature queue:

- runtime/session hardening;
- workspace references;
- causality and trace UX;
- git and review basics;
- Tool Registry v1;
- Plugin Registry v1;
- first external integrations;
- visual artifact system;
- dynamic UI from agents;
- task-based sandbox runtime;
- hybrid managed sessions;
- team/cloud model;
- expansion beyond software development.

The exact order and size of each slice should be refined in
[`feature-queue.md`](feature-queue.md).
