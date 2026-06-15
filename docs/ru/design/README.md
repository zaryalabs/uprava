# Дизайн-фаза Cortex

Статус: `draft`

Этот раздел нужен для глубокой проработки **подходов** к ключевым направлениям Cortex.

Здесь дизайн означает не список внутренних модулей, таблиц и API, а иерархическую работу над большими продуктово-архитектурными решениями:

1. Сначала понять, какой подход вообще выбираем для направления.
2. Затем раскрыть концептуальную модель: какие роли, границы, сценарии и компромиссы появляются.
3. После этого довести направление до технического дизайна: контракты, состояния, протоколы, хранение, UI, MVP cut.

Один и тот же design doc может начинаться как approach note, а затем постепенно становиться полным дизайном направления.

## Что такое направление дизайна

Направление дизайна - это крупная область продукта, где нужно выбрать принцип реализации, а не просто описать очередной модуль кода.

Примеры таких направлений:

- distributed architecture: как Cortex реализует Core / Node Daemon / clients модель;
- modular UI: как устроена модульная рабочая поверхность, блоки, панели и расширяемость интерфейса;
- plugins and Tool Registry: как подключаются tools, plugins, integrations, MCP, native adapters and visual blocks;
- dynamic UI: как агент может вернуть форму, dashboard, graph, embedded view или другой интерактивный блок;
- visualization system: какие визуализации нужны продукту и как они становятся first-class artifacts;
- trace and review: как делать работу агента проверяемой, а не просто логировать все подряд;
- execution modes: как соотносятся persistent session, task-based run and hybrid workflow.

Внутри каждого направления может быть много технических решений, но сначала нужно выбрать саму модель.

## Как писать design docs

Файлы в этой директории лучше именовать по направлению:

```text
docs/ru/design/001-distributed-architecture.md
docs/ru/design/002-modular-ui.md
docs/ru/design/003-plugins-tool-registry.md
docs/ru/design/004-dynamic-ui.md
```

Рекомендуемая структура документа:

```text
# Название направления

Статус: draft / working-position / accepted / superseded

## Зачем это нужно
## Что именно проектируем
## Что не проектируем сейчас
## Продуктовые сценарии
## Вдохновение и аналоги
## Варианты подхода
## Сравнение подходов
## Рабочая позиция
## Последствия для архитектуры
## Последствия для UI
## Последствия для агентов
## Последствия для plugins/tools/integrations
## MVP cut
## Later
## Открытые вопросы
```

Когда подход становится понятным, в этот же документ можно добавлять более низкий слой:

- термины и сущности;
- lifecycle and state machines;
- C4 / activity / sequence diagrams;
- контракты между Core, Node, UI, agent and plugin;
- формат artifacts, events, commands and permissions;
- storage and migration implications;
- failure modes and security boundaries.

Главное правило: сначала фиксируем **почему и какой подход**, потом **как именно это будет собрано технически**.

## Сквозные принципы

Эти принципы должны проходить через все направления:

- Cortex - Distributed Agent OS, а не agent chat with panels.
- Agent output не равен accepted work.
- Продукт должен снижать стоимость review, handoff, return и ownership decision.
- Модульность является архитектурным принципом, а не маркетплейсом поверх монолита.
- Visual artifacts и dynamic UI - способ понимания и управления работой, а не декоративный слой.
- Интеграции должны быть visible and traceable: не скрытые API calls внутри текста агента.
- Persistent, task-based and hybrid modes должны быть режимами одной модели, а не тремя разными продуктами.
- Human UI и agent-readable UI должны развиваться вместе.
- Stage 1 должен быть маленьким, но не должен закрывать путь к plugins, visual blocks, task-based runtime, mobile and team/cloud.

## Карта направлений

| ID | Направление | Главные вопросы подхода | Ожидаемый результат |
| --- | --- | --- | --- |
| A-001 | Distributed architecture | Как именно реализуем distributed модель? Что является Core/control plane, что остается за Node Daemon/data plane, как клиенты работают через Core, где проходят границы host/node/workspace/session? | Рабочая позиция по Core / Node Daemon / clients модели, deployment profiles and responsibility boundaries. |
| A-002 | Execution modes | Как связать persistent session, task-based sandbox run and hybrid managed session? Что общее в модели, а что различается в lifecycle, context, isolation and review contract? | Единая концепция execution modes, чтобы Stage 1 persistent workbench не заблокировал будущий task runtime. |
| A-003 | Modular UI and work surface | Что значит модульный UI для Cortex? Это Notion-like blocks, IDE/workbench panels, Obsidian-like navigation, plugin-rendered surfaces или гибрид? Какой стек и composition model подходят для web UI? | Подход к рабочей поверхности: layout, blocks, panels, navigation, extension points and constraints for React/Vite UI. |
| A-004 | Dynamic UI from agents | Как агент должен возвращать форму, dashboard, chart, graph, embedded tool или custom block? Это schema-driven UI, prebuilt block types, sandboxed components, generated code or plugin-owned renderer? | Концепция dynamic UI: что агент может породить сам, что должно быть заранее зарегистрировано, где граница безопасности. |
| A-005 | Visualization and artifacts | Какие визуализации нужны Cortex: diff, terminal replay, trace timeline, test report, UML, charts, dashboards, dependency graphs, forms? Как они соотносятся с artifacts, blocks and plugins? | Продуктовая и техническая карта visual artifacts, включая first release vs later. |
| A-006 | Plugins, Tool Registry and MCP strategy | Где живет Tool Registry? Нужен ли Core-level MCP gateway/proxy? Или MCP должен быть на уровне Node Daemon, agent process, plugin adapter, external provider? Как сравнить MCP, native adapters and hybrid adapters? | Выбранный подход к tools/plugins/integrations: registry, execution location, routing, permissions, trace and visual output. |
| A-007 | Integration UX | Как интеграции вроде GitHub, GitLab, Linear, Notion, Grafana, Docker, observability providers появляются в UI? Это tools, pages, artifacts, blocks, workflow hooks или все сразу? | Подход, при котором интеграции становятся first-class user experience, а не только tool calls. |
| A-008 | Developer workbench | Как должен ощущаться первый developer workflow: project, session, chat, terminal, files, diff, trace, checks, review? Что является главным UX-объектом: project work surface, session, task or node? | Концепция Stage 1 Developer Node Workbench и его минимальной рабочей поверхности. |
| A-009 | Trace, review and provenance | Какой trace помогает принять работу, а какой превращается в логовый шум? Как связать events, decisions, changed files, commands, checks, artifacts and risks? | Подход к review-friendly trace, audit trail and accepted-work lifecycle. |
| A-010 | Workflow and harness | Как проектировать длинные агентские задачи: durable workflow state, event-driven state machine, semi-deterministic pipelines, wakeups, CI loops, review debt? | Концепция harness/runtime для долгой работы, сначала как future constraint, позже как отдельный design. |
| A-011 | Human-agent dual interface | Как сделать UI понятным агенту? Что такое machine-readable UI state, context entry points, internal Cortex agent, long-press/chat over UI element? | Подход к dual interface, где человек и агент работают с одной видимой моделью. |
| A-012 | Security, permissions and trust | Как задавать границы для terminal, files, tools, credentials, local Node secrets, external integrations and generated UI? Как показывать risk to user? | Принципы security model, которые потом разложатся на permissions, audit and sandbox boundaries. |
| A-013 | Mobile continuity | Какой мобильный сценарий нужен первым: monitoring, unblock, review, stop/continue, answer blocking questions? Достаточно ли responsive web? | Реалистичный подход к mobile continuity без попытки сразу сделать полный mobile IDE. |
| A-014 | Metrics, observability and evals | Что измеряем: speed, review cost, iterations to merge, accepted work quality, attention/token economics, autonomous progress? Нужен ли LLM proxy? | Подход к AgentOps metrics and evals, привязанный к качеству принятой работы. |
| A-015 | Deployment and bootstrap | Как запускать Cortex: local single-user, personal distributed, team/cloud? Нужен ли compose generator/CLI bootstrap? Как это влияет на продуктовую модель? | Подход к deployment profiles and developer setup, без преждевременного усложнения. |
| A-016 | Beyond software development | Как модель переносится на analytics, research, documents, finance and knowledge workflows? Что должно быть общим, а что domain-specific? | Ограниченная стратегия расширения, чтобы не распылить Stage 1, но сохранить широту Agent OS. |

## Первый проход

Ближайшие design docs лучше делать в таком порядке:

1. `A-001 Distributed architecture`
2. `A-003 Modular UI and work surface`
3. `A-006 Plugins, Tool Registry and MCP strategy`
4. `A-004 Dynamic UI from agents`
5. `A-005 Visualization and artifacts`
6. `A-008 Developer workbench`
7. `A-009 Trace, review and provenance`
8. `A-002 Execution modes`

Причина порядка: distributed architecture уже во многом сформулирована и может стать примером формата. Затем нужно разобраться с тем, что делает Cortex не просто backend+chat: модульный UI, plugins/tools, dynamic UI and visual artifacts. После этого можно точнее спроектировать первый developer workbench and trace/review loop.

## Уровни глубины

Для каждого направления фиксируем глубину явно.

### Level 1. Approach

Документ отвечает:

- какую проблему направления решаем;
- какие варианты подхода существуют;
- какие аналоги изучаем;
- какие trade-offs видим;
- какая рабочая позиция сейчас выглядит сильнее.

### Level 2. Conceptual design

Документ добавляет:

- основные сущности;
- границы ответственности;
- пользовательские сценарии;
- agent-facing сценарии;
- UI consequences;
- first release vs later.

### Level 3. Technical design

Документ доводит направление до реализации:

- lifecycle and state machines;
- API/protocol contracts;
- artifact/event formats;
- storage implications;
- permissions and failure modes;
- tests/evals/checklist.

Не каждое направление сразу должно доходить до Level 3. На текущем этапе важнее получить Level 1 по всем крупным направлениям и Level 2/3 по тем, которые критичны для Stage 1.

## Что считать готовым результатом этой фазы

Дизайн-фаза будет полезной, если после нее будет понятно:

- какие подходы Cortex выбирает для distributed architecture, modular UI, plugins/tools, dynamic UI and visual artifacts;
- какие решения обязательны для Stage 1, а какие только накладывают constraints на архитектуру;
- какие идеи из Notion/Obsidian/IDE/Grafana/MCP/AgentOps мы берем, а какие не берем;
- где проходит граница между product concept, architecture and implementation detail;
- какие документы нужно переводить в `docs/en`, когда позиция стабилизируется.

## Открытые вопросы верхнего уровня

- Должен ли dynamic UI быть частью modular UI system или отдельным artifact/runtime слоем?
- Нужно ли начинать с набора фиксированных block types или сразу проектировать plugin-rendered blocks?
- Должен ли Core быть MCP gateway/proxy, или лучше держать MCP ближе к Node/agent/plugin execution?
- Где граница между plugin, integration, tool, block and artifact?
- Как изучить Notion-like modularity practically: как data model, как UI composition, как plugin model или как interaction pattern?
- Какие visualizations являются must-have для Stage 1, а какие только демонстрируют будущую силу платформы?
- Как не сделать слишком абстрактную платформу до появления рабочего developer workbench?
