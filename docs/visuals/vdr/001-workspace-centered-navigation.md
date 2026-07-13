# VDR-001: Workspace-centered navigation

Статус: `implemented-0.2.6`

Дата: `2026-07-13`

## Контекст

Предыдущий visual baseline смешивал разные уровни продукта:

- `Nodes` и `Jobs` были отдельными глобальными экранами;
- workspace сразу открывался как длинный файловый inspector;
- agent session запускалась из заголовка файловой поверхности;
- editor, terminal, command runner, diff и history располагались вертикально;
- постоянный правый Inspector занимал место даже без выбранного reference.

Из-за этого не читалась основная продуктовая иерархия:

```text
Node
-> Workspace
-> способы работы внутри workspace
```

`Job` уже привязан к placement/workspace, agent session работает в том же
контексте, а file/terminal tools являются рабочей средой этого workspace.

## Решение

### Информационная архитектура

Реализация `0.2.6` использует следующую иерархию:

```text
Dashboard
Nodes
  Node
    Workspace
      Agent
      Workbench
      Jobs
```

- `Dashboard` остаётся единственным глобальным продуктовым экраном.
- Левая панель является одновременно списком нод и навигацией по ним.
- Нажатие на ноду открывает её `Node Overview`; отдельный глобальный экран
  `Nodes` больше не нужен.
- Внутри ноды показываются её workspace — корневые рабочие папки.
- Нажатие на workspace открывает последнюю использованную поверхность, а в
  прототипе по умолчанию открывает `Agent`.
- `Jobs` перестают быть глобальным разделом и становятся workspace-scoped
  поверхностью.

### Названия поверхностей workspace

Для production Web Control Panel выбраны:

- **Agent** — sessions, timeline, runtime state, approvals и composer;
- **Workbench** — file tree, editor/diff и terminal;
- **Jobs** — определения фоновых jobs, расписания и run history.

`Workbench` передаёт IDE-like композицию, но не обещает полноценную IDE.
`Developer Environment` не используется, потому что может смешиваться с Node
и runtime environment. `Tasks` пока не используется, чтобы не смешивать
Background Jobs с будущим task-based runtime.

### Layout

Desktop shell:

```text
┌───────────────┬─────────────────────────────────────┐
│ Nodes         │ Node Overview или Workspace         │
│  Node         │ [ Agent ] [ Workbench ] [ Jobs ]    │
│   Workspace   │                                     │
└───────────────┴─────────────────────────────────────┘
```

Inspector для source/evidence/cause закрыт по умолчанию. Он открывается по
reference и не резервирует постоянную ширину основной поверхности.

Левая панель `Nodes -> Workspaces` также скрывается целиком через toggle в
topbar. В закрытом состоянии остаётся узкая кнопка возврата навигации, а
основная поверхность занимает освободившуюся ширину. Состояние sidebar не
зависит от открытия контекстного Inspector.

Workbench использует IDE-like композицию:

```text
┌──────────────┬──────────────────────────────────────┐
│ File Tree    │ Editor / Diff                        │
├──────────────┴──────────────────────────────────────┤
│ Terminal tabs                                       │
└─────────────────────────────────────────────────────┘
```

Отдельный постоянный `Command` block удаляется. Произвольные команды
выполняются в terminal. Будущие traceable checks могут появиться как toolbar,
command palette или отдельный компактный view, но не как дублирующая terminal
панель.

### Dashboard

Dashboard сокращается до четырёх операционных показателей:

- Core API status;
- Reachable Nodes;
- Active Runtimes;
- Running Jobs.

Ниже показывается компактная `Recent Activity` projection. Она содержит
значимые события и ссылки на объекты, но не заменяет raw event log.

### Статусы

Состояния визуально разделяются по смыслу:

- presence: `online`, `stale`, `offline`;
- lifecycle: `active`, `stopped`, `running`;
- attention: `blocked`, `approval required`, `error`;
- workspace: `clean`, `dirty`, `conflict`.

Один объект может иметь несколько состояний из разных измерений без создания
противоречивой строки равнозначных badges.

## Последствия

- Agent session получает естественную точку входа внутри workspace.
- Files и terminal становятся одной workbench-поверхностью, а не длинной
  страницей независимых карточек.
- Jobs находятся рядом с workspace и session evidence, которые они создают.
- Node Overview становится агрегированным состоянием конкретной ноды.
- Центральная поверхность получает больше места.
- Sidebar можно временно скрыть для editor, terminal или review-сценария.
- Causality Inspector остаётся общим для Agent, Workbench, Jobs и activity.

## Альтернативы

Рассматривались следующие названия:

| Вариант | Первая панель | Вторая панель | Третья панель |
| --- | --- | --- | --- |
| Выбранный | Agent | Workbench | Jobs |
| Технический | Sessions | Files & Terminal | Jobs |
| Более широкий | Agent | Developer Environment | Tasks |

Отдельные верхнеуровневые `Nodes` и `Jobs` отклонены, потому что они дублируют
иерархию Node -> Workspace и разрывают связанные рабочие поверхности.

## Вне текущего visual slice

- production-ready runtime gating и полная матрица disabled actions;
- mobile navigation;
- полный accessibility и copy polish;
- resizable/dockable panels;
- полноценная IDE, LSP и debugger;
- полный mobile navigation redesign.

## Проверка решения

Реализация позволяет визуально пройти сценарии:

1. открыть Dashboard и увидеть четыре global metrics;
2. выбрать Node и увидеть Node Overview;
3. выбрать Workspace и попасть в Agent;
4. переключиться между Agent, Workbench и Jobs без смены workspace context;
5. открыть source/evidence/cause в контекстном Inspector;
6. выполнить визуальный переход из Recent Activity к связанному объекту.

Решение проверено 13 июля 2026 года unit/component tests, mocked Playwright E2E,
desktop golden snapshots `1440x1000`, narrow desktop `1024x900`, mobile
regression `390x844` и keyboard path. Real-profile smoke остаётся отдельной
дополнительной проверкой при доступном controlled-development Core/Node/Codex
профиле.
