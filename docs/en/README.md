# English Documentation

This directory is the English-facing documentation mirror.

The Markdown document set should match [`../ru`](../ru). If a document exists in
one language tree, the other language tree should also contain the same relative
path. If product or architecture content conflicts, the Russian version has
priority and this English-facing mirror should be updated to match it.

Some deep design documents may first be mirrored from Russian source text before
they are fully translated. This keeps the documentation set complete while the
English prose is polished incrementally.

Current documents:

- [`vision.md`](vision.md) - product vision.
- [`architecture.md`](architecture.md) - Core / Node Daemon / Clients architecture.
- [`v01.md`](v01.md) - first usable product version.
- [`feature-queue.md`](feature-queue.md) - ranked implementation queue.
- [`product-evolution.md`](product-evolution.md) - product evolution model.
- [`product-stages.md`](product-stages.md) - superseded historical stage model.
- [`tech-stack.md`](tech-stack.md) - preliminary technical stack.
- [`feature-inventory.md`](feature-inventory.md) - feature and direction inventory.
- [`workspace-inspector.md`](workspace-inspector.md) - Project Workspace Inspector direction.
- [`workspace-editing-and-ide-sidecar.md`](workspace-editing-and-ide-sidecar.md) - lightweight file editing and optional full-IDE sidecar direction.
- [`uprava-notes.md`](uprava-notes.md) - source notes mirror.
- [`design/`](design/) - deep design documents for key Uprava mechanisms.

Temporary implementation plans live outside the language mirror in
[`../tmp-plans`](../tmp-plans). They are tactical working documents for active
intermediate development slices. Durable decisions discovered there should be
promoted back into the synchronized English and Russian documentation.
