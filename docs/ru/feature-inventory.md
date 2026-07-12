# Uprava Feature Inventory

Статус: `active`

Назначение: зафиксировать идеи фич, которые уже были придуманы в `README.md` и `docs/ru/uprava-notes.md`, без приоритизации и без превращения в roadmap. Это сырой, но сгруппированный инвентарь для последующего отбора в vision, архитектуру и backlog.

Источники:

- `README.md`
- `docs/ru/uprava-notes.md`

## Как читать

- Это не список обязательств.
- Повторяющиеся идеи сведены в один пункт.
- В `Источник` указаны места, откуда идея пришла или где она повторяется.
- Если идея пока выглядит как исследовательское направление, это явно отмечено.

## 1. Platform / Distributed Agent OS

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-001 | WorkOS общего назначения для агентов | Uprava как рабочая операционная среда для агентской работы, сначала для разработки, затем для аналитики, ресерча, финансов и других задач. | `README.md:3`, `docs/ru/uprava-notes.md:24` |
| F-002 | Distributed Agent OS | Система не замыкается на одной машине или одном чате, а управляет агентами, нодами, окружениями и workflow. | `docs/ru/uprava-notes.md:24` |
| F-003 | Core / control plane | Центральный слой управления агентами, проектами, нодами, задачами, артефактами и workflow. | `README.md:28`, `docs/ru/uprava-notes.md:23`, `docs/ru/uprava-notes.md:54` |
| F-004 | Node Daemon | Системный демон, который запускается на ноде, регистрирует ее в Core, запускает агентов и дает доступ к файлам и состоянию системы. | `README.md:28`, `docs/ru/uprava-notes.md:23`, User clarification, 2026-06-15 |
| F-005 | Multi-node execution | Возможность запускать агентов на разных нодах: локальный компьютер, сервер, cloud workspace, sandbox. | `README.md:17`, `docs/ru/uprava-notes.md:23`, `docs/ru/uprava-notes.md:54` |
| F-006 | Подключение личного компьютера как ноды | Можно подключить личный комп и смотреть, что там делают агенты. | `docs/ru/uprava-notes.md:23` |
| F-007 | Легковесный runtime для агентов | Среда, в которой можно быстро запускать агентов под конкретные задачи. | `docs/ru/uprava-notes.md:28` |
| F-008 | Stable isolated environment | Агент работает в стабильном окружении с песочницей, файлами, кодом, bash и UI-видимостью. | `docs/ru/uprava-notes.md:54` |
| F-009 | microVM для агентов | Все агенты могут работать в microVM. | `docs/ru/uprava-notes.md:15` |
| F-010 | Отдельная git-ветка на агента/run | Агентская работа изолируется отдельной веткой. | `docs/ru/uprava-notes.md:15` |
| F-011 | Stateless agent + sandbox | Агент не обязательно является долгоживущим процессом; работа идет через sandbox/workspace. | `docs/ru/uprava-notes.md:112-126` |
| F-012 | Durable workflow state | Долгоживущим является состояние workflow, а не контейнер или конкретная агентская сессия. | `docs/ru/uprava-notes.md:121-126` |
| F-013 | Event-driven state machine для агента | Агентская работа моделируется как event-driven state machine. | `docs/ru/uprava-notes.md:109-110` |
| F-014 | State store + event log | Хранение состояния и истории событий через Postgres/Redis/S3/Vector DB или аналогичный слой. | `docs/ru/uprava-notes.md:102-103` |
| F-015 | Deployment repo / compose generator | Репа или CLI для старта инфраструктуры: выбрать конфигурацию core/node и собрать compose/deploy config. | `docs/ru/uprava-notes.md:73` |
| F-016 | Cloud product with accounts/projects | Коммерческий cloud-вариант с аккаунтами и проектами. | `docs/ru/uprava-notes.md:55` |
| F-017 | Мультиплатформенность | Продукт должен работать между desktop и mobile. | `README.md:7`, `README.md:16`, `docs/ru/uprava-notes.md:9`, `docs/ru/uprava-notes.md:69` |
| F-018 | Execution-mode neutral core | Core не должен быть привязан только к task-based cloud-agent flow; persistent, task-based and hybrid режимы должны быть разными execution modes одной системы. | User clarification, 2026-06-15 |
| F-019 | Hybrid managed session | Живая сессия или orchestration agent может порождать bounded task runs для отдельных подзадач, а затем возвращать результат в общий workflow/session state. | User clarification, 2026-06-15 |

## 2. Agents

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-020 | Codex как default agent | Начальный агент по умолчанию. | `README.md:47` |
| F-021 | Agent orchestrator | Оркестратор, который управляет несколькими агентами и распределяет задачи. | `docs/ru/uprava-notes.md:4`, `docs/ru/uprava-notes.md:14` |
| F-022 | Internal Uprava agent | Агент внутри самого UI как first-class citizen для работы над Uprava и помощи пользователю. | `README.md:29` |
| F-023 | Internal agent orchestrates node agents | Внутренний агент может оркестрировать агентов, запущенных на нодах. | `README.md:29` |
| F-024 | Multi-chat with agents | Возможность одновременно общаться с несколькими агентами. | `docs/ru/uprava-notes.md:13` |
| F-025 | Pluggable agent providers | Единый интерфейс поверх разных агентских провайдеров, как метаинструмент для агентов. | `docs/ru/uprava-notes.md:76` |
| F-026 | CLI-коннекторы агентов | Поддержка CLI-коннекторов для разных агентских инструментов. | `docs/ru/uprava-notes.md:32` |
| F-027 | Agent server | Собственный или готовый agent server, способный выполнять задачи уровня code actions. | `docs/ru/uprava-notes.md:32` |
| F-028 | Specialized agents | Разные классы агентов: coding, support, retrieval, browser, finance. | `docs/ru/uprava-notes.md:95-100` |
| F-029 | Агенты как исполнители коммитов | По максимуму делать агентами даже коммиты. | `docs/ru/uprava-notes.md:27` |
| F-030 | Self-creation as eval | Самосоздание первой версии Uprava как оценочный сценарий. | `docs/ru/uprava-notes.md:8` |
| F-031 | Агент с доступным UI-контекстом | Интерфейс проектируется так, чтобы агент понимал, что видит пользователь. | `docs/ru/uprava-notes.md:83` |
| F-032 | Человек/агент co-working model | Возможная иерархия: люди, управляющие, работяги, агенты; либо упрощенная модель человек/агент. | `docs/ru/uprava-notes.md:10` |
| F-033 | Agents in tool environments | Направление: давать агентам окружения в виде инструментов, а не только процесс/машину. | `docs/ru/uprava-notes.md:29` |
| F-034 | Persistent agent session | Агент запускается как живой процесс или подключается как внешний interactive agent; пользователь может продолжать диалог, смотреть состояние и управлять процессом. | User clarification, 2026-06-15 |
| F-035 | Attach/detach to live agent | Возможность подключаться к уже живому агентскому процессу и отключаться без потери состояния. | User clarification, 2026-06-15 |
| F-036 | Task-based agent server mode | Агент вызывается как сервер/исполнитель задачи: ему передаются tools, context package and sandbox, а он возвращает bounded result. | User clarification, 2026-06-15 |
| F-037 | Agent CP-like connection | Возможность подключаться к агенту как к управляемому процессу/контрольной плоскости, а не только запускать отдельную задачу. | User clarification, 2026-06-15 |

## 3. Tasks, Workflows, Harness

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-040 | Долгие agent tasks | Возможность ставить агенту задачи на часы и возвращаться к результату. | `README.md:8`, `README.md:20`, `README.md:39` |
| F-041 | Harness для долгих задач | Слой практик и механизмов, помогающий агентам выполнять длинные задачи управляемо. | `README.md:39` |
| F-042 | Полудетерминированные pipelines | Workflow вида реализация -> review -> исправление, где часть этапов может быть детерминирована. | `README.md:42` |
| F-043 | Agent self-review / tests внутри pipeline | Внутри реализации агент может делать свой review и запускать тесты, но отдельный review-блок остается явным. | `README.md:42` |
| F-044 | Schedules / n8n-like automation | Пайплайны и расписания в стиле n8n, но для агентской работы. | `docs/ru/uprava-notes.md:11` |
| F-045 | Библиотека guides | Гайды, которые агент может применять, например setup Python project. | `README.md:40` |
| F-046 | Библиотека guidelines | Правила по code style, архитектуре, тестам, review и частым ошибкам агентов. | `README.md:41` |
| F-047 | Skills, tools, pipelines из коробки | Базовые готовые наборы, чтобы не настраивать каждый workflow с нуля. | `README.md:43` |
| F-048 | Task -> MR/PR flow | Сценарий, где агентская работа заканчивается merge request / pull request. | `README.md:9`, `docs/ru/uprava-notes.md:20`, `docs/ru/uprava-notes.md:120` |
| F-049 | Git webhook wakes workflow | Workflow может засыпать и просыпаться от GitHub webhook, например после CI. | `docs/ru/uprava-notes.md:121-124` |
| F-050 | CI follow-up loop | Агент проверяет CI и обновляет внешнюю задачу после webhook. | `docs/ru/uprava-notes.md:121-124` |
| F-051 | One-shot vs dialogue mode experiment | Исследовать, когда лучше "1 задача = 1 запрос", а когда диалог. | `docs/ru/uprava-notes.md:57` |
| F-052 | Иерархический подход к планированию | Двигаться от масштаба к деталям, чтобы снизить когнитивную нагрузку при проектировании. | `docs/ru/uprava-notes.md:78-81` |
| F-053 | C4 + Activity Diagram + UML State Machine | Использовать C4, activity diagrams и UML state machines для проектирования Uprava и агентских процессов. | `docs/ru/uprava-notes.md:81` |
| F-054 | Отложенные сообщения в сессии | Долговечный одноразовый будущий turn существующей сессии; он проходит обычную runtime/session admission, а не становится повторяющейся автоматизацией. | User clarification, 2026-07-12 |
| F-055 | Background Jobs и scheduled agent runs | Долговечное Job definition с task prompt, параметрами, schedules, stop-on-error policy и наблюдаемыми runs для unattended agent work в текущем workspace. | User clarification, 2026-07-12 |
| F-056 | Provider quota admission | Общая best-effort проверка пятичасового и недельного Codex limits перед chat и Job starts; порог 5%, typed rejection, explicit force override и честное состояние `unknown`. | User clarification, 2026-07-12 |

## 4. Developer Workflow

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-060 | Project/file browser | Возможность смотреть весь проект и файлы, а не только измененные файлы. | `README.md:19`, `docs/ru/uprava-notes.md:31`, `docs/ru/uprava-notes.md:59` |
| F-061 | Terminal view / agent output screen | Экран с выводом агентов, желательно как терминалы. | `docs/ru/uprava-notes.md:12`, `docs/ru/uprava-notes.md:54` |
| F-062 | Bash/tool call visibility | Видеть, какие команды и tool calls выполнялись в окружении. | `docs/ru/uprava-notes.md:54` |
| F-063 | Diff viewer | Удобная работа с diff как обязательный ранний слой. | `README.md:19`, `docs/ru/uprava-notes.md:22` |
| F-064 | Git integration | Интеграция с git providers и рабочими ветками. | `README.md:36`, `docs/ru/uprava-notes.md:20`, `docs/ru/uprava-notes.md:120` |
| F-065 | PR/MR comments import | Загрузка комментариев к PR/MR в Uprava. | `docs/ru/uprava-notes.md:20` |
| F-066 | Fix PR comments with agents | Возможность отправить PR/MR comments агенту на исправление. | `docs/ru/uprava-notes.md:20` |
| F-067 | Mobile review | Удобное review с телефона. | `README.md:10`, `docs/ru/uprava-notes.md:69` |
| F-068 | Test/check reports | Агент запускает tests через bash tool, а система показывает результат как часть run. | `docs/ru/uprava-notes.md:119` |
| F-069 | API-level regression for UI/system evals | Для benchmark UI тестировать сложно, но API можно покрывать большим regression набором. | `docs/ru/uprava-notes.md:41` |
| F-070 | Project state view | Видеть состояние проекта/задачи/агентов, а не только chat transcript. | `README.md:19`, `docs/ru/uprava-notes.md:31`, `docs/ru/uprava-notes.md:59` |

## 5. UI, Visual Artifacts, Interaction

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-080 | Notion-like block UI | Интерфейс с блоками, в которые можно встраивать разные типы данных и действий. | `README.md:30`, `docs/ru/uprava-notes.md:34` |
| F-081 | Obsidian-like knowledge/navigation model | Вдохновение от Obsidian: связность, дерево, docs, ссылки, knowledge base. | `docs/ru/uprava-notes.md:34`, `docs/ru/uprava-notes.md:72` |
| F-082 | Dynamic block | Агент может выдать форму, график или целый dashboard. | `README.md:31` |
| F-083 | Dynamic UI in chats | В чатах агент может показывать Grafana, другие инструменты и интерактивные UI-блоки. | `docs/ru/uprava-notes.md:21`, `docs/ru/uprava-notes.md:54` |
| F-084 | Visual plugins | Плагины, которые отображают действия и результаты в интерфейсе. | `README.md:38` |
| F-085 | Forms instead of text | Иногда агент должен показывать форму, а не просить/отвечать текстом. | `README.md:18`, `README.md:31` |
| F-086 | Graphs / charts | Визуализации данных, графики и другие chart artifacts. | `README.md:18`, `README.md:31` |
| F-087 | Dashboards | Агент или plugin может создавать целые dashboards. | `README.md:31`, `docs/ru/uprava-notes.md:54` |
| F-088 | Embedded external views | Встроенные views и ссылки на инструменты вроде Grafana, сервисов и внешних систем. | `docs/ru/uprava-notes.md:31`, `docs/ru/uprava-notes.md:54` |
| F-089 | UML visualization | Минимум просмотр UML. | `README.md:11` |
| F-090 | UML editor | Расширение UML visualization до редактора. | `README.md:11` |
| F-091 | Canvas / dynamic interfaces | Канвас и динамические интерфейсы как важное направление. | `docs/ru/uprava-notes.md:33` |
| F-092 | @mentions | Возможность упоминать файл, инструмент, агента и другие сущности через `@`. | `README.md:35` |
| F-093 | Dual UI | У каждого элемента есть визуальное представление для человека и машинно-читаемое представление для агента. | `README.md:32-34` |
| F-094 | Long press to internal agent chat | Долгое нажатие открывает чат с внутренним агентом для гибкого взаимодействия с UI. | `README.md:34`, `docs/ru/uprava-notes.md:83` |
| F-095 | UI доступен агенту | Агент понимает, что видит пользователь, и может действовать с учетом интерфейсного контекста. | `docs/ru/uprava-notes.md:83` |
| F-096 | Visual stack integration | Интеграции должны проявляться визуально, а не только как текст "я сделал X" или ссылка. | `README.md:12`, `docs/ru/uprava-notes.md:31` |

## 6. Integrations and Plugins

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-100 | Plugin system | Система расширений для инструментов, визуализаций, агентов, integrations и workflows. | `README.md:21`, `README.md:36-38`, `docs/ru/uprava-notes.md:63` |
| F-101 | Notion integration | Подключение Notion через plugin/integration. | `README.md:36` |
| F-102 | GitLab integration | Подключение GitLab. | `README.md:36` |
| F-103 | Linear integration | Linear как основной task tracker на старте. | `README.md:36`, `README.md:50` |
| F-104 | Grafana integration | Встроенные views, dashboards и monitoring через Grafana. | `README.md:36`, `docs/ru/uprava-notes.md:21`, `docs/ru/uprava-notes.md:30-31` |
| F-105 | Docker integration | Интеграция с Docker/deployment/runtime слоем. | `README.md:36`, `docs/ru/uprava-notes.md:73` |
| F-106 | MLflow integration | Подключение MLflow как возможного plugin. | `README.md:36` |
| F-107 | MCP integration | Подключение MCP, но с выводом результатов в визуальный UI, а не только в текст. | `README.md:12`, `docs/ru/uprava-notes.md:100` |
| F-108 | CLI access to connected tools | CLI с доступом к инструментам, подключенным на уровне системы. | `README.md:37` |
| F-109 | External task trackers instead of own tracker first | На старте не делать свой task tracker, а использовать готовые. | `README.md:48`, `README.md:50` |
| F-110 | Task tracker provider abstraction | Позже можно сделать метаинструмент поверх разных task trackers. | `docs/ru/uprava-notes.md:76` |
| F-111 | External memory instead of own memory first | На старте не делать свою memory system. | `README.md:48` |
| F-112 | Memory provider abstraction | Метаинструмент поверх разных memory providers. | `docs/ru/uprava-notes.md:5`, `docs/ru/uprava-notes.md:19`, `docs/ru/uprava-notes.md:76` |
| F-113 | Git provider integration | Интеграция с git для PR/MR, comments, branches, checks. | `docs/ru/uprava-notes.md:20`, `docs/ru/uprava-notes.md:120-124` |
| F-114 | Observability provider integration | LangSmith, Langfuse, OpenTelemetry, Phoenix или аналогичные инструменты. | `docs/ru/uprava-notes.md:105-106` |
| F-115 | Sandbox/devbox providers | Возможные внешние sandbox providers из useful links: Daytona, E2B, Sandcastle и аналоги. | `docs/ru/uprava-notes.md:132`, `docs/ru/uprava-notes.md:138-139` |
| F-116 | Core Tool Registry | Реестр tools/capabilities в Core: metadata, schemas, permissions, routing, UI contracts and audit policy. Выполнение tool может происходить на Node, во внешнем provider или позже в Core для lightweight tools. | User clarification, 2026-06-15 |
| F-117 | Plugin Registry | Реестр установленных plugins в Core: версии, конфигурация, exposed tools, visual blocks, artifact types, workflow templates, permissions and compatibility. | User clarification, 2026-06-15 |
| F-118 | Integration adapter model | Интеграции подключаются через MCP, native API, Node-local adapters, external provider adapters or hybrid adapters. MCP важен, но не является единственным вариантом. | User clarification, 2026-06-15 |
| F-119 | First-class integration UX | Интеграции должны давать UI blocks, artifacts, workflow hooks, trace and permissions, а не только скрытый tool call внутри текстового ответа агента. | User clarification, 2026-06-15 |

## 7. Traceability, Monitoring, Metrics

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-120 | Traceability / process visibility | Видеть, что агент сделал, какие файлы смотрел, какие изменения внес, какие команды запускал и что проверял. | `README.md:19`, `docs/ru/uprava-notes.md:31`, `docs/ru/uprava-notes.md:54` |
| F-121 | Review-friendly trace | Система должна уменьшать стоимость review, а не просто копить logs. | `README.md:19`, `docs/ru/uprava-notes.md:31`, `docs/ru/uprava-notes.md:61` |
| F-122 | Monitoring layer | Мониторинг агентской работы и состояния системы. | `docs/ru/uprava-notes.md:30`, `docs/ru/uprava-notes.md:61`, `docs/ru/uprava-notes.md:105-106` |
| F-123 | Explainable AI direction | Исследовать explainability/интерпретируемость агентских решений как часть будущей системы. | `docs/ru/uprava-notes.md:61` |
| F-124 | Stats screen | Экран статистики по агентам/задачам/системе. | `docs/ru/uprava-notes.md:16` |
| F-125 | LLM proxy | Возможный proxy для LLM, чтобы собирать статистику, контролировать расходы и наблюдать вызовы. | `docs/ru/uprava-notes.md:16` |
| F-126 | Hard metrics | "Железные" метрики вместо ощущения прогресса. | `docs/ru/uprava-notes.md:17`, `docs/ru/uprava-notes.md:67` |
| F-127 | Edits per iteration | Мерить число правок в одной итерации. | `docs/ru/uprava-notes.md:25` |
| F-128 | Iterations to merge | Мерить число итераций до merge. | `docs/ru/uprava-notes.md:25` |
| F-129 | Scalability/support metrics | Размеры diff, число измененных строк на функцию, число измененных модулей на функцию, число fix commits. | `docs/ru/uprava-notes.md:26` |
| F-130 | Attention budgeting / token economics | Метрики и оптимизация затрат внимания/токенов: retrieval cost, context entropy, cache stability, semantic locality. | `docs/ru/uprava-notes.md:65` |
| F-131 | Workflow provenance / audit trail | Сохранение происхождения действий, событий, проверок и решений. | `docs/ru/uprava-notes.md:102-106`, `docs/ru/uprava-notes.md:126` |
| F-132 | Review debt visibility | Видимость накопленного долга review/integration, чтобы агентская скорость не превращалась в скрытую нагрузку. | `README.md:20`, `docs/ru/uprava-notes.md:25-26`, `docs/ru/uprava-notes.md:67` |

## 8. Mobile and Collaboration

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-140 | Desktop/mobile continuity | Можно начать работу на компьютере и продолжить с телефона. | `README.md:7`, `README.md:16`, `docs/ru/uprava-notes.md:9`, `docs/ru/uprava-notes.md:69` |
| F-141 | Mobile task monitoring | С телефона видно состояние задач, агентов и их результатов. | `README.md:7`, `docs/ru/uprava-notes.md:69` |
| F-142 | Mobile review | С телефона можно ревьюить изменения, читать trace, смотреть diff и принимать решения. | `README.md:10` |
| F-143 | Multi-user control / co-working | Исследовать коллективные сценарии управления агентами по аналогии с Figma/Zed. | `docs/ru/uprava-notes.md:10` |
| F-144 | Agent work surface for teams | Общая рабочая поверхность, где видны агентские runs, задачи, результаты и статусы. | `docs/ru/uprava-notes.md:10`, `docs/ru/uprava-notes.md:54` |

## 9. Knowledge Base, Documentation, Research

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-160 | Knowledge base mode | Uprava может быть не только runtime, но и layer для базы знаний. | `docs/ru/uprava-notes.md:72` |
| F-161 | Git repo + Obsidian model | База знаний как git repo с docs, indexes, links and tree navigation. | `docs/ru/uprava-notes.md:72` |
| F-162 | Docs as code | Документы развиваются как код: создаются по потребности, делятся при росте, не плодятся заранее. | `docs/ru/uprava-notes.md:44-51` |
| F-163 | README as project/key feature source | Описание проекта и ключевых фич должно жить в основном README. | `docs/ru/uprava-notes.md:45-46` |
| F-164 | Architecture tree | Расширенная схема/дерево с коротким описанием каждого большого модуля. | `docs/ru/uprava-notes.md:51` |
| F-165 | Zotero-inspired research features | Взять полезные идеи из Zotero для research/document workflows, не превращая Uprava в Zotero. | `docs/ru/uprava-notes.md:74` |
| F-166 | Research/article workflows | Не замыкаться на разработке: статьи и исследования тоже должны поддерживаться. | `docs/ru/uprava-notes.md:24` |

## 10. Benchmarks and Evals

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-180 | Self-build benchmark | Оценивать Uprava через попытку создать систему заново с нуля. | `docs/ru/uprava-notes.md:8`, `docs/ru/uprava-notes.md:36-42` |
| F-181 | Detailed spec input benchmark | Бенчмарк с детальной спецификацией на входе. | `docs/ru/uprava-notes.md:38` |
| F-182 | Business case coverage | Большое покрытие бизнес-кейсами в eval/regression наборе. | `docs/ru/uprava-notes.md:39` |
| F-183 | Autonomous progress metric | Считать, насколько система смогла продвинуться без вмешательства человека. | `docs/ru/uprava-notes.md:40` |
| F-184 | API regression benchmark | Делать большой regression на уровне API, потому что UI тестировать сложнее. | `docs/ru/uprava-notes.md:41` |
| F-185 | Agent mode benchmark | Сравнивать single-shot, dialogue, hierarchical и pipeline режимы агентской работы. | `docs/ru/uprava-notes.md:57`, `docs/ru/uprava-notes.md:78-81` |
| F-186 | Execution mode comparison | Сравнивать persistent session, task-based sandbox run и hybrid managed session по review cost, autonomy, latency, trace quality and user control. | User clarification, 2026-06-15 |

## 11. Domains and Expansion

| ID | Фича / направление | Смысл | Источник |
| --- | --- | --- | --- |
| F-200 | Software development first | Первый фокус - разработка, потому что там есть project, files, git, tests, diff, review, MR/PR. | `README.md:3`, `docs/ru/uprava-notes.md:22`, `docs/ru/uprava-notes.md:59` |
| F-201 | Analytics workflows | Дальнейшее расширение на аналитику. | `README.md:3` |
| F-202 | Research workflows | Дальнейшее расширение на ресерч, статьи и исследования. | `README.md:3`, `docs/ru/uprava-notes.md:24` |
| F-203 | Finance workflows | Дальнейшее расширение на финансы. | `README.md:3`, `docs/ru/uprava-notes.md:98` |
| F-204 | Personal tasks branch | Возможно отдельное ответвление для личных задач. | `README.md:3` |
| F-205 | Site/email generators | Легковесный агентский runtime может упростить генерацию сайтов и писем. | `docs/ru/uprava-notes.md:28` |
| F-206 | Knowledge workflows | Процессы базы знаний, docs, indexes, research library и handoff через repo/tree. | `docs/ru/uprava-notes.md:72`, `docs/ru/uprava-notes.md:74` |

## 12. Candidate Foundation Cut

Если из этого инвентаря выделять первый практический слой, наиболее связная минимальная версия выглядит так:

- Core/control plane.
- Node Daemon.
- Project + workspace.
- Codex as default agent.
- Persistent agent session.
- Chat plus non-chat views.
- File browser.
- Terminal/output view.
- Diff view.
- Basic session event log and trace.
- Basic git/diff awareness.
- Mobile-readable run/review state.
- Minimal dynamic artifact/block API.
- Minimal plugin boundary.
- Minimal Tool Registry and Plugin Registry shape.

Task-based sandbox runs, durable workflows and full MR/PR flow are intentionally outside the first foundation cut. The first product should prove the persistent Node-based developer workbench and leave architectural space for task-based mode later.

Эта версия проверяет главный тезис: Uprava дает больше управляемости, визуализации и traceability, чем обычный agent chat.
