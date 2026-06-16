# Дизайн-фаза Cortex

Статус: `draft`

Этот раздел нужен для глубокой проработки **ключевых механик** Cortex.

Здесь дизайн означает не список внутренних модулей, таблиц и API, а иерархическую работу над большими продуктово-архитектурными решениями:

1. Сначала сформулировать vision ключевой механики: суть идеи, продуктовую логику и почему это важно для Cortex.
2. Затем развернуть vision в architecture: сущности, границы, сценарии, lifecycle, контракты, artifacts/events, хранение, permissions, UI consequences и проверку качества.

Один и тот же design doc начинается с корневого vision-блока, а затем постепенно разворачивается в architecture-блок. Внутри architecture глубина может расти от концептуальной модели до технических контрактов.

## Что такое ключевая механика

Ключевая механика - это крупный продуктово-архитектурный механизм Cortex, который задает модель работы пользователя, агента, UI и backend-системы.

Это не отдельный модуль кода, не частная feature и не пользовательский flow. Внутри ключевой механики может быть много модулей, сценариев, UI-состояний, событий и технических решений, но сама механика описывает, **как работает важная часть системы**.

Примеры ключевых механик:

- distributed architecture: как Cortex реализует Core / Node Daemon / clients модель;
- distributed runtime coordination: как Core координирует runtime work на конкретных Node/workspace placements, dispatch, events, stale/offline, resource warnings and overrides;
- modular UI: как устроена модульная рабочая поверхность, блоки, панели и расширяемость интерфейса;
- plugins and Tool Registry: как подключаются tools, plugins, integrations, MCP, native adapters and visual blocks;
- dynamic UI: как агент может вернуть форму, dashboard, graph, embedded view или другой интерактивный блок;
- visualization system: какие визуализации нужны продукту и как они становятся first-class artifacts;
- go to cause / causality navigation: как пользователь переходит от результата, diff, ошибки или artifact к причине;
- run mode: как Cortex запускает агентскую работу через Persistent Runtime, stateless/ephemeral runtime или hybrid strategy, и как поверх этого различаются interactive session and bounded task contracts;
- human-agent dual interface: как человек и агент работают с одной видимой моделью, где agent является first-class citizen.

Внутри каждой ключевой механики может быть много технических решений, но сначала нужно выбрать саму модель.

## Как писать design docs

Файлы в этой директории лучше именовать по ключевой механике:

```text
docs/ru/design/001-distributed-architecture.md
docs/ru/design/002-run-mode.md
docs/ru/design/003-distributed-runtime-coordination.md
docs/ru/design/004-modular-ui-work-surface.md
docs/ru/design/005-dynamic-ui-from-agents.md
```

Рекомендуемая структура документа:

```text
# Название ключевой механики

Статус: draft / working-position / accepted / superseded

## Vision

### Какую проблему решает механика
### Концептуально как реализуем
### Пользовательские сценарии
### Agent-facing сценарии
### First release vs later

## Architecture

### Основные сущности
### Границы ответственности
### UI consequences
### Detailed lifecycle and state machines
### API/protocol contracts
### Artifact/event formats
### Storage implications
### Permissions and failure modes
### Tests/evals/checklist
```

Главное правило: сначала фиксируем **vision и суть механики**, потом разворачиваем это в **architecture**. Сценарии и UI появляются уже в `Vision`, но детальные состояния, контракты, storage and permissions живут в `Architecture`.

## Сквозные принципы

Эти принципы должны проходить через все ключевые механики:

- Cortex - Distributed Agent OS, а не agent chat with panels.
- Agent output не равен accepted work.
- Продукт должен снижать стоимость review, handoff, return и ownership decision.
- Модульность является архитектурным принципом, а не маркетплейсом поверх монолита.
- Visual artifacts и dynamic UI - способ понимания и управления работой, а не декоративный слой.
- Интеграции должны быть visible and traceable: не скрытые API calls внутри текста агента.
- Persistent, stateless/ephemeral and hybrid strategies должны быть runtime strategies одного Run Mode, а не разными продуктами.
- Distributed Runtime Coordination должен быть общим dispatch/resource-awareness слоем для interactive sessions, future task runs and sandboxed runtimes.
- Human UI и agent-readable UI должны развиваться вместе.
- Stage 1 должен быть маленьким, но не должен закрывать путь к plugins, visual blocks, task-based runtime, mobile and team/cloud.

## Карта ключевых механик

| ID | Механика | Ключевые вопросы механики | Ожидаемый результат |
| --- | --- | --- | --- |
| A-001 | Distributed architecture | Как именно реализуем distributed модель? Что является Core/control plane, что остается за Node Daemon/data plane, как клиенты работают через Core, где проходят границы host/node/workspace/session? | Рабочая позиция по Core / Node Daemon / clients модели, deployment profiles and responsibility boundaries. |
| A-002 | Run Mode | Что такое Run Mode как единая механика запуска агентской работы? Как устроены Persistent Runtime, stateless/ephemeral runtime and hybrid strategy? Как поверх runtime strategy различаются interactive session and bounded task contracts? Где границы между project, workspace, node, thread, turn, run and agent process? | Концепция Run Mode для Stage 1 и дальше: Persistent Runtime first, managed process lifetime, lifecycle, visible surface, review points and constraints for future stateless/sandboxed strategies. |
| A-003 | Distributed Runtime Coordination | Как Core координирует runtime work между session thread, runtime session, workspace placement and Node? Как dispatch-ятся commands, как упорядочиваются events, как UI видит node/workspace tree, stale/offline, resource warning badges and overrides? Как git repo/branch signals подсвечивают возможные конфликты без lock-системы? | Рабочая модель coordination layer для Stage 1: Nodes -> Projects/Workspaces tree, command proxy, idempotency, event ordering, resource signals, warning badges, override events and reuse by future task/sandbox runtimes. |
| A-004 | Modular UI and work surface | Что значит модульный UI для Cortex? Это Notion-like blocks, IDE/workbench panels, Obsidian-like navigation, plugin-rendered surfaces или гибрид? Где проходят границы pages, panels, blocks, artifacts, integration surfaces and extension points? | Модель рабочей поверхности: layout, blocks, panels, navigation, plugin surfaces and constraints for React/Vite UI. |
| A-005 | Dynamic UI from agents | Как агент должен возвращать форму, dashboard, chart, graph, embedded tool или custom block? Это schema-driven UI, prebuilt block types, sandboxed components, generated code or plugin-owned renderer? | Концепция dynamic UI: что агент может породить сам, что должно быть заранее зарегистрировано, где граница безопасности. |
| A-006 | Visualization and artifacts | Какие визуализации нужны Cortex: diff, terminal replay, causality map, test report, UML, charts, dashboards, dependency graphs, forms? Как они соотносятся с artifacts, blocks and plugins? | Продуктовая и техническая карта visual artifacts, включая first release vs later. |
| A-007 | Plugins, Tool Registry and MCP strategy | Где живет Tool Registry? Нужен ли Core-level MCP gateway/proxy? Или MCP должен быть на уровне Node Daemon, agent process, plugin adapter, external provider? Как сравнить MCP, native adapters and hybrid adapters? | Модель tools/plugins/integrations: registry, execution location, routing, permissions, events and visual output. |
| A-008 | Go to cause and causality UX | Как сделать аналог go to definition, но для причинности работы агента? Как из diff line, failed check, artifact, decision, status или UI block перейти к породившему prompt/context/tool call/command/event/file change? Что является cause graph, а что просто log noise? | Модель UIUX причинности: навигация от результата к причине, минимальная модель cause links and evidence without dumping raw trace. |
| A-009 | Human-agent dual interface and Agent as First-Class Citizen | Как сделать UI понятным и человеку, и агенту? Что такое machine-readable UI state, context entry points, internal Cortex agent, chat over UI element, agent identity, capabilities, status, memory, permissions and ownership? | Модель dual interface, где agent является видимым участником системы, а не скрытым процессом за текстовым чатом. |

Не все важные темы являются отдельными ключевыми механиками. Некоторые стоит держать как пользовательские сценарии или срезы внутри design docs:

- Developer workbench - главный Stage 1 сценарий для `A-002 Run Mode`, `A-003 Distributed Runtime Coordination`, `A-004 Modular UI and work surface` and `A-008 Go to cause and causality UX`.
- Workflow and harness - сценарный срез для длинной работы, который проверяет `A-002 Run Mode` and `A-003 Distributed Runtime Coordination`, но не заменяет их.
- Integration UX - частный случай модульности, plugins/tools and visual blocks; его нужно раскрывать внутри `A-004`, `A-005`, `A-006` and `A-007`.
- Security, permissions and trust - обязательный architecture-срез для execution modes, plugins/tools, dynamic UI and agent identity, но не отдельная ключевая механика карты.
- Metrics, observability and evals - quality/feedback-срез для проверки механик, а не самостоятельная ключевая механика.
- Mobile continuity, deployment/bootstrap and beyond software development - важные constraints/product horizons, но не ключевые механики текущей design-фазы.

## Глубина проработки

Для каждой ключевой механики сначала нужен `Vision`. Это не summary, а корневой смысловой блок: какую проблему решаем, какую модель предлагаем, какие человеческие и agent-facing сценарии считаем ключевыми, что попадает в first release and later.

`Architecture` можно заполнять постепенно. Не каждая ключевая механика сразу должна доходить до подробных state machines, API contracts, storage implications or tests/evals. На текущем этапе важно создать design doc по каждой механике из карты, зафиксировать в каждом `Vision`, а затем углублять `Architecture` там, где решение критично для Stage 1 или блокирует соседние механики.

## Что считать готовым результатом этой фазы

Дизайн-фаза будет полезной, если после нее будет понятно:

- по каждой ключевой механике из карты есть design doc с корневым `Vision` и заготовкой `Architecture`;
- как устроены ключевые механики Cortex: distributed architecture, run mode, distributed runtime coordination, modular UI, plugins/tools, dynamic UI and visual artifacts;
- какие решения обязательны для Stage 1, а какие только накладывают constraints на архитектуру;
- какие идеи из Notion/Obsidian/IDE/Grafana/MCP мы берем, а какие не берем;
- где проходит граница между product concept, architecture and implementation detail;
- какие документы нужно переводить в `docs/en`, когда позиция стабилизируется.

## Открытые вопросы верхнего уровня

- Должен ли dynamic UI быть частью modular UI system или отдельным artifact/runtime слоем?
- Нужно ли начинать с набора фиксированных block types или сразу проектировать plugin-rendered blocks?
- Должен ли Core быть MCP gateway/proxy, или лучше держать MCP ближе к Node/agent/plugin execution?
- Где граница между plugin, integration, tool, block and artifact?
- Как изучить Notion-like modularity practically: как data model, как UI composition, как plugin model или как interaction pattern?
- Какие visualizations являются must-have для Stage 1, а какие только демонстрируют будущую силу платформы?
- Какой минимальный cause graph нужен для go to cause, чтобы помогать review, но не превращаться в шумный trace log?
- Как именно agent должен быть представлен в UI как first-class citizen: identity, status, permissions, memory, capabilities или отдельный work object?
- Как внутри Run Mode развести interactive session contract и bounded task contract так, чтобы они использовали общую модель project/workspace/node/agent/artifact/event?
- Как не сделать слишком абстрактную платформу до появления рабочего developer workbench?
