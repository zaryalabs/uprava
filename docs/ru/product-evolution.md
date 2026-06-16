# Эволюция продукта Cortex

Статус: `draft`

Этот документ заменяет старую модель стадий. Каноническое описание первой
версии теперь находится в [`v01.md`](v01.md), а порядок следующих работ - в
[`feature-queue.md`](feature-queue.md).

## Рабочая модель

- `V01` - первая осязаемая версия продукта: что пользователь сможет запустить,
  открыть, увидеть, потрогать и проверить.
- `Feature Queue` - очередь реализационных срезов, отсортированная по
  зависимости, сложности, риску и продуктовой ценности.
- `Feature Inventory` - полный инвентарь известных идей и направлений без
  обещания порядка реализации.
- Design docs - целевая модель ключевых механик без жесткой временной нарезки.

## V01

Первая версия продукта - **Developer Node Workbench**:

- Core Backend и Web Control Panel;
- одна или несколько нод с Node Daemon;
- persistent Codex-backed session через Agent Provider Adapter;
- project/workspace binding;
- chat/session view;
- Project Workspace Inspector: file tree, file viewer, lightweight text editor,
  workspace terminal/PTY sessions, command/output history, basic diff/check
  entry points;
- basic trace and event log;
- минимальная форма Tool Registry, Plugin Registry and visual block/artifact
  contract.

V01 проверяет тезис:

```text
Persistent Runtime + Node Daemon + Core UI
дают больше контроля, continuity and reviewability,
чем обычный локальный agent chat.
```

## Дальше

Дальнейшее развитие не фиксируется как линейная цепочка стадий. Вместо этого
следующие срезы должны выбираться из очереди фич:

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

Точный порядок и размер каждого среза должны уточняться в
[`feature-queue.md`](feature-queue.md).
