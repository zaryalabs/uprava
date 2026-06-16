# Cortex Vision

Статус: `draft`

## Короткая формула

Cortex - это Distributed Agent OS для массовой работы с агентами.

Продукт должен стать рабочей операционной средой, в которой человек управляет агентами как распределенными workloads: запускает их на разных нодах, видит их окружение, контролирует ход работы, проверяет результат, принимает изменения и получает не только текстовый ответ, но и интерактивные визуальные артефакты.

Если текущие агентские инструменты чаще всего выглядят как чат с ограниченным доступом к окружению, Cortex должен быть control plane для агентской работы: проекты, ноды, агенты, файлы, терминалы, диффы, задачи, workflow, плагины, артефакты и след решений.

## Product Thesis

ИИ-агенты становятся достаточно сильными, чтобы выполнять длинные задачи, но интерфейсы вокруг них остаются слишком плоскими. Человек видит чат и финальный ответ, но плохо видит рабочую среду, изменения, источники, проверки, риски, промежуточные решения и состояние задачи.

Cortex решает это не через "еще один чат", а через Agent OS:

- снизу - Docker/Kubernetes-подобная модель нод, node daemons, рабочих окружений и запускаемых agent workloads;
- сверху - Notion/Obsidian-подобная рабочая поверхность с блоками, связями, визуализациями и живыми артефактами;
- между ними - модульный runtime, который соединяет агентов, инструменты, проекты, task trackers, git, MCP, плагины и интерфейсы проверки.

## Проблема

Современные агентские инструменты закрывают только часть работы:

- чат показывает диалог, но не дает полноценного взгляда на проект, файловую систему, терминал и состояние окружения;
- результат агента часто сводится к тексту "я сделал X", ссылке или diff view без богатого контекста;
- долгие задачи плохо управляются: сложно поставить задачу на часы, уйти, вернуться и быстро понять, что произошло;
- review и integration остаются ручной нагрузкой, а инструмент почти не помогает снизить стоимость проверки;
- мобильный сценарий слабый: можно читать чат, но сложно полноценно управлять работой, ревьюить, смотреть файлы, проверять состояние и принимать решения;
- интеграции существуют как tools/connectors, но их результаты редко становятся first-class UI;
- агентская работа плохо трассируется: человек отвечает за результат, но часто не видит достаточного следа для проверки, handoff, возврата и отката.

## Видение продукта

Cortex должен стать системой, где агентская работа имеет управляемую форму.

Человек не просто пишет prompt и ждет ответ. Он выбирает проект, ноду, агента, режим выполнения, workflow, допустимые границы, ожидаемые доказательства и критерии приемки. Иногда это живой агентский процесс, к которому человек подключается и с которым работает интерактивно. Иногда это task-based запуск в изолированном окружении. Иногда это гибрид, где постоянная сессия порождает bounded runs для отдельных подзадач.

Агент работает в наблюдаемом окружении. Система показывает ход работы, изменения, проверки, риски и артефакты. Результат становится принимаемым work item только после review, correction, integration и ownership decision.

Ключевая идея:

```text
agent output != accepted work

accepted work = output + validation + correction + integration + ownership decision
```

Cortex должен делать этот цикл дешевле, прозрачнее и удобнее.

## Модель системы

Базовая модель Cortex:

- **Core / Control Plane** - центральный слой управления проектами, нодами, агентами, задачами, workflow, артефактами, правами и состоянием.
- **Node** - зарегистрированное вычислительное окружение, где может выполняться работа: локальный компьютер, сервер, devbox, microVM, cloud workspace или sandbox.
- **Node Daemon** - системный демон на ноде, который регистрирует ноду в Core, запускает агентов, управляет workspace, дает доступ к файлам, терминалу, процессам, логам и состоянию.
- **Project** - рабочий контекст: репозиторий, документы, настройки, агенты, интеграции, history, workflows, artifacts.
- **Workspace** - конкретное окружение выполнения задачи: checkout, branch, sandbox, mounted files, env, running tools.
- **Agent Session** - живой агентский процесс или подключение к внешнему агенту, к которому можно attach/detach, продолжать диалог, смотреть состояние и управлять окружением.
- **Agent Run** - ограниченный эпизод работы агента с целью, scope, контекстом, событиями, логами, изменениями, проверками и результатом.
- **Execution Mode** - способ выполнения агентской работы: persistent session, task-based run или hybrid mode.
- **Workflow** - долговечное состояние работы, которое может переживать перезапуск агента, контейнера или ноды.
- **Artifact** - результат работы агента, который может быть текстом, diff, файлом, dashboard, UML, формой, отчетом, графиком, embedded tool или кастомным UI-блоком.
- **Tool Registry** - реестр tools/capabilities в Core: metadata, schemas, permissions, routing, UI contracts and audit policy.
- **Plugin** - расширение, которое добавляет agents, tools, integrations, visual blocks, workflows, commands или новые типы артефактов.

Эта модель должна позволить начать с developer workflow, но не замкнуться на нем.

Подробнее client/server модель зафиксирована в [architecture.md](architecture.md).

## Режимы выполнения

Cortex не должен быть привязан к одному cloud-agent flow. Task-based sandbox подход важен, но это только один режим. Минимальная модель должна поддерживать как минимум два режима, а в перспективе - гибрид между ними.

### Persistent agent session

Агент запускается как живой процесс или подключается как внешний interactive agent. Пользователь может подключиться к нему, продолжать диалог, смотреть терминал/логи/файлы, давать уточнения и управлять процессом почти как рабочей сессией.

Этот режим подходит для:

- exploratory work;
- совместного проектирования;
- задач, где контекст уточняется по ходу;
- работы с локальной нодой;
- случаев, где важна интерактивность и continuity.

Ключевые свойства:

- attach/detach к живому агенту;
- долгоживущий контекст процесса;
- видимость файлов, терминала, команд и текущего состояния;
- ручное управление ходом работы;
- trace как журнал сессии и важных решений, а не только финальный report.

### Task-based sandbox run

Агент вызывается как исполнитель задачи. Core передает ему цель, tools, context package, sandbox/workspace, критерии и expected evidence. Агент работает изолированно, возвращает результат, trace, изменения, проверки и артефакты.

Этот режим похож на cloud agents и хорошо подходит для:

- bounded implementation tasks;
- background work на часы;
- CI/fix/review loops;
- воспроизводимых workflow;
- задач, где нужна изоляция, branch, sandbox или microVM;
- случаев, где проще проверять результат как пакет изменений.

Ключевые свойства:

- bounded task input;
- sandbox/tool environment;
- event log;
- explicit stop condition;
- review-ready output;
- durable workflow state вместо привязки к живому процессу.

### Hybrid managed session

Гибридный режим соединяет постоянную интерактивную сессию и task-based подзапуски. Пользователь работает с живым агентом или orchestration agent, а тот может создавать изолированные task runs для отдельных подзадач: проверить гипотезу, сделать diff, запустить CI-fix, подготовить артефакт, провести review.

Такой режим может быть близок к тому, как ощущаются современные cloud coding agents, но с большей прозрачностью: пользователь видит и управляющую сессию, и отдельные bounded runs, которые она порождает.

Ключевой вопрос для дизайна: где проходит граница между живым контекстом сессии и воспроизводимым state/trace отдельных task runs.

## Принципы

### 1. Distributed Agent OS, not chat app

Чат - важный интерфейс, но не центр системы. Центр системы - управляемая агентская работа: где она выполняется, что ей разрешено, какие файлы и инструменты затронуты, какие проверки пройдены, что изменилось и как результат принять или откатить.

### 2. Execution-mode neutral core

Core должен моделировать агентскую работу так, чтобы persistent sessions и task-based runs были разными режимами одной системы, а не двумя разными продуктами. Общими должны быть проекты, ноды, workspaces, tools, files, artifacts, trace, permissions, review и integrations. Различаться должны lifecycle, isolation, state ownership and review contract.

### 3. Модульность как архитектурная основа

Cortex не должен пытаться сразу заменить Linear, GitHub, GitLab, Notion, Grafana, Docker, Temporal, sandbox providers, memory systems и все MCP servers. Сильная позиция продукта - быть runtime, агрегатором и интерфейсным слоем, который соединяет эти системы и делает их доступными человеку и агенту через единый рабочий контур.

Плагины и интеграции должны быть first-class частью архитектуры. Core хранит Tool Registry and Plugin Registry: какие capabilities доступны, откуда они пришли, кто может их использовать, где они исполняются, как они отображаются в UI, как трассируются and какие artifacts/workflows они добавляют. MCP - важный путь интеграции, но не единственный.

### 4. Visualization-first output

Агентский результат не должен ограничиваться текстом. Если результат лучше воспринимается как diff, таблица, график, форма, UML, dashboard, timeline, terminal replay, test report, dependency graph или embedded external view, Cortex должен уметь показать его как first-class artifact.

Визуализация - не декоративный слой. Это способ снизить стоимость понимания, review и принятия решения.

### 5. Traceability by default

Значимые агентские задачи должны оставлять читаемый след: цель, scope, контекст, ограничения, ключевые решения, использованные файлы/источники, проверки, результаты, unresolved risks, измененные артефакты, next step и решение reviewer.

Trace нужен не для бюрократии. Он снижает стоимость review, возврата к задаче, handoff и интеграции:

```text
trace -> lower review cost + lower return cost + better handoff + reusable memory
```

Принцип дозировки важен: маленькая задача может оставить 2-4 строки, большая задача требует отдельного trace artifact, zone map или review note. Слишком слабый trace заставляет человека восстанавливать контекст заново. Слишком тяжелый trace никто не будет читать.

### 6. Прозрачность и право вмешаться

Человек может отвечать за результат только если у него есть context, authority, resources and ability to intervene. Cortex должен показывать не только финальный output, но и то, что было делегировано, что агент сделал, что проверено, что не проверено, где риски и как остановить, исправить или откатить действие.

Практический тест: человек, который не участвовал в диалоге с агентом, должен понять, что было делегировано, что принято и что остается рискованным, не спрашивая агента заново.

### 7. Human-agent dual interface

Интерфейс должен быть удобен человеку и доступен агенту. UI-элементы, артефакты, статусы и действия должны иметь машинно-читаемое представление, чтобы внутренний агент мог понимать, что видит пользователь, помогать с навигацией, объяснять состояние и действовать с учетом UI-контекста.

### 8. Durable workflows over long-lived containers

В task-based режиме долговечным должно быть состояние workflow, а не обязательно конкретный процесс, контейнер или агентская сессия. Агент может быть перезапущен, workspace может быть пересоздан, нода может смениться, но система должна помнить, где работа остановилась, какие решения приняты, какие проверки нужны и что является return trigger.

В persistent режиме долгоживущий процесс допустим как first-class execution mode, но он все равно должен иметь observable state, attach/detach semantics and trace.

### 9. Integration over reinvention

Сначала Cortex должен соединять лучшие готовые элементы: git providers, task trackers, MCP, observability, sandbox runtimes, workflow engines, dashboards, memory tools. Свои реализации нужны там, где общий интерфейс, связность, UX или traceability невозможно получить интеграцией.

### 10. Mobile continuity

Работа с агентами должна продолжаться между компьютером и телефоном. Мобильный сценарий должен позволять не только читать сообщения, но и понимать состояние задач, смотреть trace, просматривать diff, принимать простые review decisions, останавливать агент, отвечать на blocking questions и возвращать задачу в работу.

### 11. Superadditive work

Cortex должен усиливать человека, а не вытеснять его из процесса. Цель - не максимальная автономия любой ценой, а такая связка человека, агентов, интерфейса и следа решений, где растут скорость, качество, понимание, навык и способность безопасно делегировать.

## Первый слой продукта

Первая версия должна заложить фундамент Distributed Agent OS, не пытаясь сразу реализовать все направления.

Минимальный foundation:

- Core с projects, nodes, agent sessions, runtimes, messages and events;
- Node Daemon на ноде, который умеет запускать Codex-backed runtime and report
  state;
- привязка agent session к node, project and workspace;
- один реализованный execution mode: persistent interactive session;
- task-based and hybrid modes сохраняются как architecture directions, а не V01
  implementation;
- chat как первый интерфейс к session;
- navigation формата `Nodes -> Projects/Workspaces -> Sessions`;
- lifecycle controls: start, attach, detach, interrupt, stop, resume and return
  later, если provider это поддерживает;
- basic status model for node, project/workspace, runtime and session;
- basic event history and diagnostics для lifecycle, offline, stale, warning and
  error states;
- trusted local/single-user or controlled development deployment, with security
  baseline as the first hardening slice after V01;
- UI shell and entity model, подготовленные для будущих file browser, terminal,
  diff, trace, tools, plugins, review and visual artifact surfaces.

Базовые developer flows:

```text
persistent:
node/project/session tree -> start or attach agent session -> chat -> lifecycle/events -> stop/resume

task-based:
future task -> agent run -> sandbox/tools -> diff -> checks -> trace -> review -> MR/PR

hybrid:
future session -> spawn bounded task runs -> review artifacts -> merge state back into session/workflow
```

Цель первого слоя - доказать, что Cortex дает больше прозрачности и управляемости, чем обычный чат с агентом.

## Развитие продукта

Каноническая первая версия продукта описана в [`v01.md`](v01.md).
Очередь следующих реализационных срезов описана в
[`feature-queue.md`](feature-queue.md). Карта возможной эволюции продукта
описана в [`product-evolution.md`](product-evolution.md).
Подробный инвентарь уже придуманных фич и направлений вынесен в
[feature-inventory.md](feature-inventory.md).

Первая версия продукта - **V01 Distributed Agent Control Panel**:

- Core Backend and Web Control Panel;
- одна или несколько нод с Node Daemon;
- persistent Codex-backed session через Agent Provider Adapter;
- navigation tree формата `Nodes -> Projects/Workspaces -> Sessions`;
- project/workspace binding как placement context;
- chat/session view как первая primary work surface;
- session lifecycle controls: start, attach, detach, interrupt, stop, resume and
  return later, если provider это поддерживает;
- basic node, project, runtime, session, message and event persistence;
- UI shell and typed command/event envelopes, подготовленные для будущих
  workspace, editor, terminal, tools, plugins, trace and artifact surfaces.

После V01 развитие лучше вести как feature queue: каждая ключевая механика
может иметь маленький полезный срез, а затем постепенно расти до целевой формы.

## Метрики успеха

Метрики должны показывать не только скорость генерации output, но и качество принятой работы:

- время от постановки задачи до review-ready результата;
- число итераций до merge / acceptance;
- доля agent runs, принятых без большого ручного переписывания;
- стоимость review: сколько времени занимает понять, проверить и принять результат;
- количество unresolved risks на момент acceptance;
- частота возвратов к задаче без потери контекста;
- средний размер review debt;
- число успешных долгих задач без постоянного участия человека;
- время разработки нового plugin/block/workflow;
- мобильная завершенность: сколько решений можно принять с телефона без перехода на desktop.

## Non-goals

На ранних этапах Cortex не должен:

- строить собственный task tracker вместо Linear;
- строить собственный git provider вместо GitHub/GitLab;
- строить полноценную memory system до проверки runtime и workflow модели;
- конкурировать с Grafana/Notion/Obsidian как самостоятельными продуктами;
- делать универсальную автоматизацию вроде n8n до появления устойчивой агентской модели;
- скрывать сложность агентской работы за красивым статусом "done".

## Открытые вопросы

- Что является минимальной единицей работы: task, agent run, workflow или artifact?
- Какой объект является верхним в UX: persistent session, task, workflow или project work surface?
- Как именно устроить hybrid mode между живой сессией и isolated task runs?
- Насколько жестко первый продукт должен быть завязан на software development?
- Делаем ли durable workflow engine своим слоем или сначала интегрируем готовый?
- Какой минимальный plugin/block API нужен уже в первой версии?
- Как выглядит trace artifact для маленькой, средней и большой задачи?
- Как отделить полезную traceability от логового шума?
- Где граница между внутренним агентом Cortex и агентами, которые работают на нодах?
- Какие visual artifacts нужны в first release: diff, terminal, UML, dashboard, forms, test report?
- Какой мобильный сценарий должен быть первым: monitoring, unblock, review или task launch?
- Какие ограничения безопасности нужны для daemon, файлов, терминала и внешних integrations?

## Рабочая позиция

Самая сильная начальная формулировка:

Cortex - это control plane и рабочая поверхность для агентских workloads. Он начинает с software development, потому что там ясны файлы, git, tests, diff, review and MR/PR flow. Но его базовые абстракции должны быть шире разработки: node, node daemon, workspace, agent session, agent run, workflow, artifact, tool registry, plugin and trace.

Если этот фундамент сделать правильно, Cortex сможет развиться не в очередной agent chat, а в модульную операционную систему для совместной работы человека и агентов.
