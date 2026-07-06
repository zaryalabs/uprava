# Релизы Uprava

Статус: `active`

Current release baseline: `0.1.5`.

Этот ledger фиксирует implementation baselines. Он не заменяет
[`feature-queue.md`](feature-queue.md), где остается ранжированная очередь
future work.

## Release Ledger

| Version | Date | Status | Completed Slice |
| --- | --- | --- | --- |
| `0.1.0` | 2026-07-06 | shipped | V01 Distributed Agent Control Panel |
| `0.1.1` | 2026-07-06 | shipped | Security baseline |
| `0.1.2` | 2026-07-06 | shipped | Runtime/session hardening |
| `0.1.3` | 2026-07-06 | shipped | Workspace shell and reference model |
| `0.1.4` | 2026-07-06 | shipped | Read-only Project Workspace Inspector |
| `0.1.5` | 2026-07-06 | current | Workspace intervention layer |

## Current Baseline

`0.1.5` включает первый working distributed control panel и пять закрытых
feature queue slices после `0.1.0`:

- controlled-development security baseline;
- runtime/session hardening;
- stable workspace/reference model;
- read-only Project Workspace Inspector;
- workspace intervention layer с text save, bounded command runner, command
  history and diff/check entry points.

Новые аудиты и temporary plans должны считать это фактами текущей реализации.
Они могут ссылаться на `V01`, когда обсуждают исторический первый продуктовый
срез.
