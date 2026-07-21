# A-012 Plugin Contribution Resolution

Статус: `working-position`

Этот документ фиксирует минимальную общую механику разрешения plugin
contributions. Она нужна до появления нескольких renderers, artifact viewers,
decorations and actions, которые могут воздействовать на один объект Uprava.

Механика не пытается исключить все конфликты. Зрелая plugin system неизбежно
допускает несовместимые или перекрывающиеся extensions. Задача Uprava — сделать
результат детерминированным, порядок управляемым, а конфликт видимым в Plugin
Panel.

## Vision

### Проблема

Plugin Registry уже умеет активировать contributions, а Web Extension Host —
подбирать renderer по source kind and surface. При появлении второго
подходящего renderer-а нельзя полагаться на порядок ответа Core, загрузки
modules или React mounting. Пользователь должен понимать:

- какие plugins воздействуют на выбранный target;
- в каком порядке применяются их contributions;
- какая contribution выбрана для exclusive extension point;
- какие contributions конфликтуют;
- как изменить порядок или отключить только проблемную contribution.

### Минимальная модель

Первый contract использует только три основных понятия:

```text
Contribution
Target
Order
```

`Contribution` — одно расширение известного extension point. `Target` —
нормализованная область, к которой оно применяется. `Order` — стабильная
последовательность подходящих активных contributions.

Универсальный язык произвольных scopes не вводится. Каждый extension point сам
определяет bounded поля своего target.

Примеры:

```text
visual.renderer:
  source_kind: chat.assistant_message
  surface: session.timeline
  render_scope: content

artifact.viewer:
  artifact_type: test-report

markdown.fragmentRenderer:
  fragment_kind: code_fence
  language_id: mermaid
```

## Architecture

### Два режима композиции

Каждый extension point определяет один из двух режимов:

```text
exclusive
  применяется первая доступная contribution в effective order

ordered
  применяются все доступные contributions в effective order
```

Режим задаёт платформа, а не plugin.

Начальные примеры:

| Extension point | Mode |
| --- | --- |
| Primary content renderer | `exclusive` |
| Artifact viewer | `exclusive` |
| Fragment replacement renderer | `exclusive` |
| Decorations | `ordered` |
| Actions | `ordered` |
| Inspector aspects | `ordered` |

Theme остаётся отдельным явным пользовательским выбором и не требует общей
очереди применения.

### Детерминированный порядок

Если пользователь не задавал preference, Host использует стабильный порядок:

```text
bundled contributions
-> остальные plugins по plugin_id
-> contributions одного plugin по contribution_id
```

Порядок не зависит от discovery timing, network response order, lazy loading
или React mounting. Установка нового plugin-а не должна молча вытеснять уже
работающий bundled default.

Пользователь может изменить порядок contributions для конкретного target.
Effective order вычисляется как:

```text
valid user order for target
-> stable default order for entries without preference
```

Для `exclusive` применяется первая enabled and available contribution, а
следующие являются fallback alternatives. Если выбранная contribution
disabled, incompatible, unavailable or failed before mount, Host пробует
следующую, затем обязательный safe fallback.

Для `ordered` Host применяет все enabled and available contributions сверху
вниз. Специальные `before`/`after`, arbitrary priorities and dependency solver
в первый contract не входят.

### Определение конфликта

В первом contract конфликт имеет узкое и проверяемое определение:

> Две активные contributions конфликтуют, если имеют одинаковый normalized
> target внутри `exclusive` extension point.

Несколько contributions для `ordered` extension point не считаются
конфликтом: это нормальная композиция.

Если позже появятся реальные случаи частичного пересечения targets, contract
можно расширить. Первый slice не вводит эвристический анализ overlap,
specificity scoring или runtime collision graph.

### Contribution identity and effective projection

Чтобы порядок и конфликт можно было объяснить, effective projection должна
сохранять provenance:

```text
EffectiveContribution:
  plugin_id
  plugin_version
  contribution_id
  extension_point
  contract_version
  target
  effective_state
  contribution
```

Порядок хранится на уровне contribution target, а не package целиком. Это
позволяет отключить или переставить renderer, не отключая unrelated theme,
action or service того же plugin-а.

### Plugin Panel

Plugin Panel должна показывать для plugin-а:

```text
Affects
  список extension points and targets

Conflicts
  другие active contributions с тем же exclusive target

Current order
  effective order для конфликтующего или ordered target
```

Минимальные действия:

- enable/disable plugin;
- enable/disable отдельную contribution;
- поднять или опустить contribution внутри target order;
- сбросить пользовательский порядок;
- перейти к конфликтующему plugin-у.

Отдельный graph UI не нужен. Достаточно conflict badge/filter в существующей
Plugin Panel и простого редактора порядка для target.

### Extension Host resolution

Общий resolver получает extension point, normalized target, effective
contributions and saved order:

```text
resolve(extension_point, target)
-> filter enabled, compatible and permitted contributions
-> apply saved or stable default order
-> exclusive: return first contribution plus alternatives
-> ordered: return all contributions
-> expose conflict metadata for Plugin Panel
```

Resolver заменяет локальный выбор первого совпадения через порядок массива.
Он должен возвращать достаточно metadata, чтобы UI мог показать текущего
победителя и альтернативы, но полноценная runtime history первого среза не
нужна.

### Markdown and nested renderers

Bundled Markdown renderer остаётся `exclusive` primary content renderer. Когда
появятся Mermaid, color or other fragment plugins, Markdown integration может
добавить отдельные bounded extension points, например
`markdown.fragmentRenderer`.

```text
assistant message
-> exclusive Markdown renderer
-> detected Mermaid fence
-> exclusive Mermaid fragment renderer
```

Первый contract не требует общего Visual AST or ContentTree. Такой слой можно
добавить позже, только если локальных typed fragment contracts станет
недостаточно.

### Generated React boundary

Generated React artifact использует свой explicit artifact/runtime target и не
конкурирует с Markdown renderer-ом. Host contributions могут воздействовать на
artifact shell, actions and metadata, но не внедряются во внутренний DOM
sandboxed iframe. Composition внутри sandbox не входит в первый slice.

## Первый implementation slice

Отдельный пункт очереди должен реализовать:

1. normalized target identity для поддерживаемых contributions;
2. platform-owned mode `exclusive` or `ordered` для extension point;
3. общий deterministic resolver;
4. provenance в effective contribution projection;
5. contribution-level enable/disable;
6. сохраняемый пользовательский order per target;
7. conflict badge/list and order controls в Plugin Panel;
8. safe fallback для unavailable exclusive chain.

Текущий Markdown renderer является первым acceptance case: второй renderer с
тем же target должен появиться как видимый конфликт, порядок должен меняться
пользователем, а результат не должен зависеть от порядка ответа Core.

## Не входит в первый contract

- универсальная scope algebra;
- dependency/conflict graph;
- `before`/`after` constraints;
- произвольный numeric priority;
- constraint solver;
- heuristic specificity ranking;
- runtime collision history;
- автоматическая композиция DOM/React trees разных plugins;
- composition внешних plugins внутри Generated React sandbox.

Эти механизмы добавляются только по подтверждённым сценариям, а не как
предварительная платформенная абстракция.

## Связь с соседними направлениями

- `A-004` определяет extension points and work surfaces; `A-012` определяет,
  как несколько contributions разрешаются для одного target.
- `A-005` использует resolution contract для Generated React runtime, artifact
  shell contributions and actions, сохраняя sandbox boundary.
- `A-006` определяет visual semantics, source refs and fallback; `A-012`
  выбирает и упорядочивает предоставляющие их renderer contributions.
- `A-007` остаётся authority для Tool Registry and plugin/tool links; callable
  tool routing не подменяется UI contribution order.

Короткая формула:

```text
Uprava допускает plugin conflicts,
но делает target, order, winner and alternatives видимыми и управляемыми.
```
