# Версионирование Uprava

Статус: `active`

Uprava использует Semantic Versioning для implementation and release baselines:

```text
MAJOR.MINOR.PATCH
```

До `1.0.0` поле `MINOR` обозначает цельный продуктовый baseline, а `PATCH`
обозначает завершенные feature or fix slices поверх этого baseline. Public
compatibility contract до `1.0.0` еще может меняться, но каждый version bump
должен делать состояние репозитория понятнее.

## Правила именования

- Используем `0.x.y` для pre-production development releases.
- Используем `0.1.0` для первой shipped working baseline.
- Увеличиваем `PATCH` для завершенных implementation slices, которые не
  переопределяют продуктовый baseline.
- Увеличиваем `MINOR`, когда product shape or architecture baseline меняется
  настолько, что downstream docs and runbooks требуют нового baseline.
- Резервируем `1.0.0` для первого production-ready compatibility and security
  contract.

## Product Cuts And Release Versions

`V01` - это имя продуктового среза, а не SemVer version. Оно описывает первый
цельный product scope, который был shipped как `0.1.0`.

Release versions описывают состояние репозитория и реализации. Current planning
documents должны ссылаться на текущий release baseline, а не на `V01`, когда
они обсуждают features, доставленные после `0.1.0`.

Current baseline: `0.1.6`.

## Правила обновления

Когда закрывается пункт feature queue или другой большой этап работы:

1. Обновить [`releases.md`](releases.md) с version and completed slice.
2. Обновить package metadata, если меняется текущая implementation version.
3. Обновить temporary plans, если они ссылаются на устаревший scope.
4. Перенести durable product, architecture or process decisions в
   синхронизированные docs under `docs/en` and `docs/ru`.

Temporary plans могут сохранять historical references to `V01`, но не должны
использовать `V01` как shorthand для текущей реализации после shipped
post-`0.1.0` slices.
