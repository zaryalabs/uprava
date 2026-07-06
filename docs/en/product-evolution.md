# Uprava Product Evolution

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

The first product version is **Distributed Agent Control Panel**:

- Core Backend and Web Control Panel;
- one or more nodes with Node Daemon;
- persistent Codex-backed session through Agent Provider Adapter;
- `Nodes -> Projects/Workspaces -> Sessions` navigation tree;
- project/workspace binding as placement context;
- chat/session view as the first primary work surface;
- session lifecycle controls: start, attach, detach, interrupt, stop, resume, and
  return later where provider support allows it;
- basic node, project, runtime, session, message, and event persistence;
- UI shell, entity model, and command/event envelopes shaped for future
  workspace, editor, terminal, tools, plugins, trace, and artifact surfaces.
- trusted local/single-user or controlled development deployment, with
  production security hardening deferred to the first post-V01 slice.

V01 validates the thesis:

```text
Persistent Runtime + Node Daemon + Core UI
turn agent chat into a distributed control surface.
```

## Next

Further development is not fixed as a linear chain of stages. Instead, the next
slices should be selected from the feature queue:

- security baseline;
- runtime/session hardening;
- workspace shell and references;
- Project Workspace Inspector;
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
