# A-002 Run Mode

Статус: `working-position`

Этот документ фиксирует первую рабочую позицию по **Run Mode** как ключевой
механике Cortex. Run Mode описывает не одну конкретную фичу, а способ, которым
Cortex запускает агентскую работу в проекте, продолжает ее, наблюдает за ней и
останавливает.

Зафиксированная позиция для V01: Cortex начинает с рабочей сессии с живым
агентским process на Node. Долговечной является **рабочая сессия**:
`SessionThread`, workspace, files, trace, diff и resume context. Сам
provider process живет между turns, но может быть остановлен после суток без
meaningful runtime steps и позже возрожден в том же workspace.

V01 начинается с **Persistent Runtime**: Node Daemon запускает Codex
provider runtime на ноде, runtime остается живым между turn-ами, а
пользователь может attach/detach к одной рабочей поверхности. Это ближе к
модели T3 Code. При этом Persistent Runtime не означает бессрочный OS process:
runtime должен иметь управляемый lifetime. V01 baseline: если у runtime
больше суток не было новых runtime steps, Node может остановить provider
process, но session thread, workspace state и resume context должны позволять
возродить процесс при возвращении пользователя. Позже Cortex может добавить
stateless/ephemeral strategies, более похожие на Codbash-like resume/launcher
подход или sandboxed task runtime.

Дополнительное уточнение: V01 является **Codex-first**, но не должен
становиться **Codex-only** в продуктовой модели. Первый adapter может быть
практично оптимизирован под Codex, но Core, UI, trace и workflow state должны
говорить на языке `AgentProvider`, `RuntimeSession`, `SessionThread`, `Turn`,
events, approvals, files, diff и trace, а не на языке Codex-specific process
details. Future adapters для OpenCode, Claude Code и других agents должны
добавляться через тот же минимальный launch/resume/control boundary, а не через
новую параллельную модель.

Ключевое уточнение: Run Mode не должен смешивать product contract и runtime
strategy.

```text
Interactive session vs bounded task = product/work contract.
Persistent vs stateless/ephemeral = runtime continuity strategy.
```

В первой версии Cortex делает interactive developer workbench через
Persistent Runtime. Task-like bounded work и stateless/sandboxed execution
остаются архитектурно возможными, но не являются V01 реализацией.

## Vision

### Какую проблему решает механика

Обычный агентский чат плохо подходит для долгой разработки. Пользователь видит
сообщения, но плохо видит реальное окружение: какие файлы открыты или
изменены, какие команды выполнялись, где агент ждет approval, какие проверки
прошли, что изменилось с последнего turn-а и можно ли безопасно продолжать
работу позже.

С другой стороны, "задачный" подход, где каждый запуск агента является
одноразовым process invocation, хорошо подходит для bounded work, но хуже
подходит для совместного проектирования, уточнений, live approvals и
ручного вмешательства.

Cortex должен иметь один Run Mode model, внутри которого можно явно выбрать
runtime strategy и work contract:

- пользователь работает в project/workspace на node;
- Cortex знает, какой runtime strategy используется;
- live work, files, output, diff, trace, approvals и review state видны в UI;
- долговечным является не только process, но и system state вокруг него;
- позже bounded tasks и stateless/sandboxed runs не требуют отдельной
  продуктовой модели.

V01 проверяет тезис: **Persistent Runtime + Node Daemon + Core UI дают
больше контроля и continuity, чем локальный agent chat или launcher поверх
логов**.

### Концептуально как реализуем

Run Mode нужно разложить на несколько слоев:

1. **Work Contract** - как пользователь понимает работу: interactive session,
   bounded task, review run, fix run, research run.
2. **Runtime Strategy** - как живет агентский runtime/process: persistent,
   stateless/ephemeral, sandboxed, external provider, hybrid.
3. **Session Thread** - долговечная история диалога, turns, activity, trace и
   review state.
4. **Workspace Binding** - конкретная папка, checkout, branch/worktree, env и
   local capabilities на выбранной Node.
5. **Runtime Session** - live provider runtime/process, если он сейчас
   запущен и управляется Node Daemon.
6. **Turn** - один пользовательский input и связанный цикл работы агента до
   `idle`, `blocked`, `interrupted`, `completed` или `error`.

В V01 основной вариант:

```text
Run Mode: Persistent Runtime
Work Contract: interactive developer session

start session -> start provider runtime on Node
turn 1        -> send message into the same runtime
turn 2        -> send message into the same runtime
attach        -> subscribe UI to existing runtime/session state
detach        -> UI disconnects, runtime can keep living
24h no steps  -> stop runtime, keep thread/workspace/resume state
return later  -> start and resume runtime in the same workspace
stop          -> explicitly stop runtime
```

Новый turn не должен стартовать новый CLI process с нуля. Он отправляется в
уже открытый runtime через provider adapter/protocol. Session thread и
workspace binding при этом должны переживать disconnect UI, transient node
issues, runtime expiry и runtime recovery attempts.

### Runtime strategies

#### Persistent Runtime

Node Daemon запускает provider runtime и держит его живым между turns в рамках
простого active runtime window. Persistent здесь означает "живет достаточно
долго для интерактивной работы", а не "живет вечно".

Свойства:

- low-latency continuation;
- streaming output;
- interactive approvals и user-input requests;
- interrupt/stop;
- live runtime status;
- provider-native session state;
- attach/detach from multiple clients through Core;
- managed lifetime with idle expiry;
- better control-plane integration than plain terminal launcher.

Это default для V01 и first Codex implementation.

#### Runtime lifetime policy

Для V01 не нужен сложный scheduler с большим набором lease-политик.
Достаточно простого baseline: runtime process является live resource на Node,
а долговечным объектом является session thread вместе с workspace и resume
state.

Рабочая позиция для V01:

- session thread, workspace binding, files, trace/diff history и provider
  resume cursor/session id долговечны;
- provider process живет только пока active runtime window не истек;
- Node Daemon хранит `last_runtime_step_at`;
- runtime step - это meaningful activity: accepted user turn, provider output,
  tool/command step, approval/user-input request, explicit checkpoint/resume
  event; heartbeat сам по себе не должен продлевать жизнь процесса;
- если runtime не выполняет активный turn и `last_runtime_step_at` старше 24
  часов, Node может остановить provider process и отправить Core событие
  `runtime.expired`;
- attached UI client не делает process бессрочным: UI может показывать, что
  runtime давно idle, и при необходимости предложить resume;
- pending approval/user-input не должен исчезать молча: UI показывает blocked
  state и expiry time; если ответа нет больше суток, Node Daemon может
  cancel/expire request, остановить runtime и записать trace event;
- explicit Stop всегда завершает provider process, но не удаляет session
  thread/workspace state;
- resurrection процесса - нормальная часть lifecycle, а не только аварийный
  recovery path.

Пример:

```text
turn completed
  -> runtime ready
  -> no runtime steps for 24h
  -> Node stops provider process
  -> Core marks runtime expired
  -> user returns later
  -> Node starts provider runtime in the same workspace
  -> Node resumes provider session by resume cursor/session id
  -> Core marks runtime ready
```

"В том же состоянии" здесь означает тот же user-visible state: session thread,
workspace, files, branch/worktree, trace, diff baseline, provider session id
и resume cursor. Cortex не должен обещать сохранение RAM/in-memory state
убитого process. Если provider-native resume невозможен, UI должен честно
показать degraded resume: work surface остается readable, а новый runtime
получает явный resume context из сохраненного thread/workspace state.

Так Cortex сохраняет главный UX Persistent Runtime - несколько turn-ов попадают
в один live process, пока идет работа, - но не превращает Node в накопление
забытых CLI processes.

#### Stateless / Ephemeral Runtime

Позже Cortex может поддержать strategy, где каждый turn, resume action или task
step стартует новый CLI/provider process с тем же `resume_id`, `cwd`, project
context и workspace binding. Долговечным остается session thread, workspace,
trace и provider history, но не OS process.

Это ближе к Codbash-like подходу: session identity и history существуют в
agent storage/logs, а продолжение может происходить через `codex resume`,
`claude --resume` или similar launch command. Такой подход полезен как fallback,
compatibility mode или task-oriented runtime, но он слабее для reliable
streaming, approvals, interrupts и structured trace, если Cortex не владеет
runtime protocol.

#### Sandboxed Runtime

Sandboxed runtime - частный случай stateless или managed runtime strategy, где
workspace создается под bounded work: отдельная папка, branch/worktree,
container, microVM или external sandbox provider. Это естественная база для
future bounded tasks, но V01 не должен начинаться с нее.

#### Hybrid Runtime

Hybrid strategy появляется позже: persistent interactive session может
порождать bounded stateless/sandboxed runs для отдельных подзадач, а результаты
возвращаются в общий thread/workflow state.

### Work contracts

#### Interactive Session Contract

Пользователь работает с живой рабочей поверхностью. Цель может уточняться по
ходу. Важны attach/detach, visible environment, approvals, intervention и
return.

Это V01 contract.

#### Bounded Task Contract

Пользователь или agent задает ограниченную работу: goal, scope, context
package, stop condition, expected evidence и review-ready output. Runtime
strategy чаще будет stateless/sandboxed, но это не обязательно.

Это later contract.

### Пользовательские сценарии

#### 1. Start live work in project

Пользователь выбирает проект и ноду, нажимает start. Core создает session
thread и workspace binding. Node Daemon запускает Codex runtime в workspace,
сообщает Core status `starting -> ready`, UI показывает chat, terminal/output,
files, diff и trace panels.

#### 2. Continue the same agent

Пользователь отправляет второе сообщение. Cortex не создает новую задачу и не
запускает новый агентский процесс. Он отправляет turn в тот же runtime
session. Agent сохраняет conversational/runtime context, а Cortex привязывает
новые events, output и file changes к новому turn.

#### 3. Detach and return later

Пользователь закрывает браузер или открывает work surface с телефона. Runtime
может продолжать жить на Node, пока `last_runtime_step_at` не ушел за 24-hour
expiry window. Новый клиент подключается к Core, получает snapshot of session
thread, latest status, diff, trace и active runtime state, затем подписывается
на live events через Core. Если runtime уже auto-stopped по 24h no-steps
policy, UI показывает stopped/expired runtime state и предлагает resume.

#### 4. Runtime expired or died but work remains

Если runtime остановился после 24 часов без runtime steps, Node Daemon
перезапустился, provider runtime упал или нода ушла в sleep, Core не теряет
session thread. UI показывает, что live runtime отсутствует, expired или stale.
Если у provider есть resume cursor/session id, Node Daemon может восстановить
runtime в том же workspace. Во время восстановления UI показывает `resuming`:
есть задержка на старт process, handshake с provider и подхват session state.
Если восстановление невозможно, work surface остается readable: chat, trace,
files и diff доступны как historical state.

#### 5. Agent asks for a decision

Когда runtime запрашивает approval, file-change approval или structured
user-input, Node Daemon нормализует request и стримит его в Core. UI
показывает blocking state и action. Ответ пользователя возвращается в тот же
live runtime.

#### 6. Queue direction: task-like launch

Позже пользователь сможет сказать "сделай bounded run из этой session":
зафиксировать goal/scope, создать isolated workspace, запустить agent step и
вернуть review-ready output обратно в общую рабочую поверхность. Это не часть
V01, но Run Mode должен не закрывать такой путь.

### Agent-facing сценарии

Для агента Run Mode должен выглядеть как стабильная рабочая среда, а не как
невидимый локальный shell:

- агент знает selected project/workspace и granted scope;
- tool access проходит через provider runtime, Node Daemon или registered tools;
- agent-visible context может включать session summary, recent turns, files,
  constraints, pending approvals и current workspace state;
- runtime не получает long-term node credentials;
- важные actions становятся events, artifacts или trace entries, а не только
  текстом в assistant response;
- если runtime восстановлен после перезапуска, агент получает достаточно
  контекста, чтобы продолжить без притворства, что process никогда не падал;
- если runtime был остановлен по 24h no-steps policy, агент должен получить
  явный resume context вместо неявной иллюзии непрерывного процесса;
- future task contract должен быть agent-readable: goal, scope, stop condition,
  expected evidence и review gate.

### First release vs later

#### V01

Для Developer Node Workbench нужно:

- Core session/thread registry;
- Node Daemon controlled Codex runtime;
- Persistent Runtime as default runtime strategy;
- runtime lifetime policy на базе `last_runtime_step_at` и 24h no-steps
  expiry;
- interactive session as default work contract;
- one project/workspace binding per session;
- thread with turns и messages;
- live event stream through Core;
- status: `starting`, `ready`, `running`, `blocked`, `expired`, `resuming`,
  `stale`, `stopped`, `error`;
- user input path for chat turns;
- approval/user-input request path;
- interrupt и stop;
- terminal/output view;
- file browser;
- basic diff per turn or since baseline;
- basic trace и event log;
- runtime started/stopped/expired events;
- attach/detach semantics;
- durable provider resume cursor/session id;
- runtime resurrection attempt when provider supports resume cursor.

V01 does not need:

- bounded task execution contract;
- general stateless sandbox runs;
- durable workflow engine;
- CI webhook wakeups;
- PR/MR automation;
- multi-agent orchestration;
- provider-neutral feature parity across every CLI agent.

V01 **does** need a minimal Provider Adapter boundary. Иначе Codex-specific
launch, resume, approval и event semantics начнут протекать в Core API, UI и
trace model, а будущие OpenCode/Claude Code adapters придется добавлять через
ломку доменной модели.

#### Later

Later versions can add:

- stateless/ephemeral runtime as compatibility/fallback strategy;
- richer lease policies, configurable TTLs, quotas и per-project runtime
  budgets;
- sandboxed runtime for bounded work;
- task contract with context package и review contract;
- hybrid sessions that spawn bounded task runs;
- richer checkpoints и rollback;
- multiple concurrent runtimes per project;
- session handoff between providers;
- mobile-first unblock/review flows;
- stronger causality graph from result to prompt/tool/command/file change.

## Architecture

### V01 implementation: Process-backed Interactive Session

Первый конкретный режим Cortex:

- runtime strategy: **Persistent Runtime**;
- work contract: **Interactive Session**;
- provider: **Codex**;
- execution location: **Node Daemon**;
- durable control plane: **Core**;
- first client: **Web Control Panel**.

Рабочее название внутри архитектуры: **Process-backed Interactive Session**.
Это живая рабочая сессия в проекте, где пользовательский thread долговечен, а
provider process живет на Node между turns, пока active runtime window не
истек.

Инварианты первого режима:

- один `SessionThread` открывается пользователем как рабочая поверхность;
- один `WorkspaceBinding` привязывает session к папке на Node;
- один active `RuntimeSession` держит live Codex process для этой session;
- новые turns идут в тот же process, пока он жив;
- отсутствие runtime steps больше 24 часов позволяет Node остановить process;
- возврат пользователя запускает resurrection через provider resume
  cursor/session id в том же workspace;
- если provider-native resume невозможен, UI показывает degraded resume, а не
  притворяется, что live state полностью восстановлен.

V01 не реализует bounded tasks, workflow engine, sandbox orchestration,
multi-agent scheduling или full generic provider platform. Но модель должна
иметь минимальный Provider Adapter boundary и быть достаточно общей, чтобы
новые agents, runtime strategies и work contracts позже добавились как
расширения, а не как отдельный продукт.

### Responsibility boundaries

#### Core

Core является durable control plane.

Core отвечает за:

- project, node, workspace binding, session thread, turn и runtime ids;
- хранение thread messages, turns, status projection и event log;
- routing команд от клиентов к нужной Node;
- подписки клиентов на session events;
- отображение последнего известного runtime state;
- хранение provider resume reference, если это безопасно и достаточно для
  восстановления;
- review-facing state: approvals, trace, diff metadata, artifacts и status.

Core не должен:

- запускать provider process напрямую;
- иметь прямой доступ к workspace files на Node мимо Node Daemon;
- считать provider process долговечным источником истины;
- полагаться на то, что UI подключен во время работы агента.

#### Node Daemon

Node Daemon является data plane и runtime owner.

Node Daemon отвечает за:

- проверку, что workspace доступен на этой Node;
- запуск, мониторинг, interrupt и stop provider process;
- provider adapter lifecycle;
- filesystem observation, local diff и command/output capture;
- нормализацию provider events в Cortex events;
- хранение local runtime handle/process metadata;
- `last_runtime_step_at` и 24h no-steps expiry;
- resurrection runtime в том же workspace через provider resume cursor/session
  id;
- graceful degradation, если provider-native resume невозможен.

Node Daemon не должен:

- владеть продуктовой историей session thread;
- принимать решения review/approval без Core/user path;
- превращать local terminal logs в единственный source of truth;
- держать забытые provider processes бесконечно.

#### Provider Adapter

Provider Adapter изолирует конкретный способ общения с provider runtime.

Первый production adapter - Codex. Future adapters могут быть OpenCode, Claude
Code или другие provider runtimes. Они могут отличаться launch command,
session identity, resume mechanism, output format, approval semantics и tool
permissions, но наружу должны отдавать нормализованные Cortex events и runtime
lifecycle.

Adapter отвечает за:

- старт runtime process в нужном `cwd`;
- создание или resume provider session;
- отправку user turns в live runtime;
- стрим provider output/events;
- interrupt/stop;
- extraction provider session id/resume cursor;
- mapping provider-specific requests на Cortex approval/user-input requests.

V01 может иметь один production adapter для Codex. Provider-neutral API
нужно держать минимальным и практичным, а не пытаться сразу покрыть все CLI
agents.

#### Web Control Panel

UI является клиентом Core, а не прямым клиентом Node.

UI отвечает за:

- session work surface: chat, output, files, diff, trace и approvals;
- attach/detach к существующей session thread;
- отображение `expired`, `resuming`, `stale`, `blocked` и `running` states;
- отправку turns, approvals, interrupts и stop через Core;
- явное объяснение пользователю, когда live runtime отсутствует и нужен resume.

### Domain objects

V01 должен различать долговечные и live объекты.

#### Project

Product-level контейнер. В V01 это может быть тонкая запись: name, repo или
folder reference, default node/workspace preferences.

#### Workspace Binding

Связка session thread с конкретным workspace на Node.

Минимальные поля:

```text
workspace_binding_id
project_id
node_id
workspace_path
git_repository_url optional
git_branch optional
baseline_ref optional
created_at
```

Workspace Binding долговечен. Если runtime process умер, binding остается.

#### Session Thread

Долговечный пользовательский thread работы.

Минимальные поля:

```text
session_thread_id
project_id
workspace_binding_id
run_mode = process_backed_interactive_session
runtime_strategy = persistent_runtime
work_contract = interactive_session
status
created_at
updated_at
last_turn_id optional
active_runtime_session_id optional
provider_resume_ref optional
```

Session Thread является главным объектом, который пользователь открывает в UI.
Он не равен OS process.

#### Runtime Session

Live-or-recoverable runtime на Node.

Минимальные поля:

```text
runtime_session_id
session_thread_id
node_id
provider = codex
provider_session_id optional
provider_resume_cursor optional
process_state
started_at
last_runtime_step_at
expired_at optional
stopped_at optional
exit_reason optional
```

Core хранит projection и identifiers. Node хранит process-local handle: pid,
transport connection, temporary sockets, local adapter state. Эти process-local
details не должны становиться Core contract.

#### Turn

Один пользовательский input и связанный цикл agent work.

Минимальные поля:

```text
turn_id
session_thread_id
seq
user_message_id
status
started_at
completed_at optional
blocked_request_id optional
```

Turn может завершиться `completed`, `blocked`, `interrupted`, `error` или
`expired`. Runtime Session может пережить много turns.

#### Runtime Step

Meaningful activity внутри runtime.

Runtime step обновляет `last_runtime_step_at`. Heartbeat, ping и passive UI
attach не являются runtime step.

Runtime step kinds для V01:

- accepted user turn;
- provider output chunk или assistant message;
- tool/command started;
- tool/command output;
- tool/command completed;
- file change observed;
- approval/user-input requested;
- approval/user-input resolved;
- checkpoint/resume event;
- runtime started/resumed/stopped/expired.

### State ownership

Core persisted state:

- projects;
- nodes и node health projection;
- workspace bindings;
- session threads;
- messages и turns;
- runtime session projection;
- provider resume reference, when available;
- event log;
- approvals и user-input requests;
- artifact metadata;
- diff/checkpoint metadata enough for review UI.

Node local state:

- workspace files;
- provider process handle;
- provider local storage/logs;
- runtime transport details;
- command output buffers before they are streamed to Core;
- local filesystem watcher state;
- local diff computation cache.

Provider state:

- provider-native conversation/runtime state;
- provider-native resume id/session id;
- provider-specific logs or storage.

Core должен считать, что состояние Node и provider может исчезнуть. Product
остается readable из Core state, а work можно возродить, когда Node/provider
может дать достаточно resume context.

### Runtime state machine

Runtime Session status - это projection, а не вся правда о provider process.

```text
absent
  -> starting
  -> ready
  -> running
  -> blocked
  -> running
  -> ready
  -> expired
  -> resuming
  -> ready
  -> stopped
```

Failure branches:

```text
starting -> error
running  -> error
running  -> stale
blocked  -> expired
resuming -> error
stale    -> resuming
```

Status meanings:

| Status | Meaning |
| --- | --- |
| `starting` | Node запускает provider process. |
| `ready` | Runtime жив и может принять новый turn. |
| `running` | Runtime выполняет turn или tool/command step. |
| `blocked` | Runtime ждет approval или structured user input. |
| `expired` | Process остановлен no-steps policy, durable state сохранен. |
| `resuming` | Node запускает process и восстанавливает provider session. |
| `stale` | Core потерял свежий контакт с Node/runtime и показывает last known state. |
| `stopped` | User или system явно остановил runtime. |
| `error` | Runtime упал и требует user-visible recovery или restart. |

`blocked` не равен `running`: agent не может продолжить, пока human или policy
не ответит. `stale` не равен `expired`: `stale` означает, что Core не уверен в
состоянии, потому что contact с Node/runtime не свежий; `expired` означает, что
Node намеренно остановила process и сообщила об этом.

### Lifecycle

#### 1. Create session

1. UI отправляет `CreateSessionThread(project_id, node_id, workspace_path)`.
2. Core проверяет project/node access и создает `SessionThread`.
3. Core создает `WorkspaceBinding`.
4. Core отправляет `StartRuntime(session_thread_id, workspace_binding)` на Node.
5. UI подписывается на session event stream через Core.

#### 2. Start runtime

1. Node проверяет local workspace path.
2. Node запускает Codex provider runtime в `workspace_path`.
3. Adapter создает или получает provider session id/resume cursor.
4. Node отправляет `runtime.starting`, затем `runtime.ready`.
5. Core сохраняет runtime projection и provider resume reference.

Если provider startup падает, Node отправляет `runtime.error` с user-safe
reason. Core сохраняет session thread readable.

#### 3. Send turn

1. UI отправляет message в Core.
2. Core создает `Message` и `Turn`.
3. Если runtime находится в `ready`, Core отправляет `SendTurn` на Node.
4. Если runtime находится в `expired`, Core сначала отправляет `ResumeRuntime`,
   затем `SendTurn`.
5. Node отправляет user input в provider adapter.
6. Node нормализует provider output в events и стримит их в Core.
7. Core обновляет thread, turn, runtime projection и review-facing views.

Каждый accepted user turn обновляет `last_runtime_step_at`.

#### 4. Stream work

Во время running turn Node отправляет event stream:

```text
turn.started
provider.output.delta
command.started
command.output.delta
command.completed
files.changed
diff.updated
approval.requested
turn.completed
```

V01 не требует идеальной trace semantics. Нужно достаточно events, чтобы
ответить на вопросы:

- что попросил user;
- что сделал agent;
- какие files изменились;
- какие commands запускались;
- где agent ждал human;
- какой result готов к review.

#### 5. Block for approval/user input

Если provider запрашивает approval или structured user input:

1. Node отправляет `approval.requested` или `user_input.requested`.
2. Core сохраняет request и помечает runtime как `blocked`.
3. UI показывает blocking control.
4. User отвечает через Core.
5. Core отправляет answer на Node.
6. Node передает answer в provider adapter.
7. Runtime возвращается в `running` или `ready`.

Blocked requests тоже обновляют `last_runtime_step_at`. Если ответа нет 24
часа, Node может expire request и остановить runtime. Core сохраняет trace
event, чтобы user видел, почему work остановилась.

#### 6. Detach and attach

Detach не является runtime operation. Это только означает, что UI client
отписался.

Attach flow:

1. UI открывает существующий `session_thread_id`.
2. Core возвращает snapshot: messages, turns, runtime status, latest diff, files
   summary, pending approvals и artifacts.
3. UI подписывается на live events.
4. Если runtime находится в `expired`, UI может предложить Resume.
5. Если runtime находится в `stale`, UI может показать reconnect/resume action
   после проверки Node health.

Attached clients не держат runtime живым навсегда. No-steps policy основана на
meaningful runtime activity, а не на присутствии браузера.

#### 7. Expire after no steps

Node периодически проверяет live runtimes:

```text
if runtime.status not in [running, starting, resuming]
and now - last_runtime_step_at > 24h:
  stop provider process
  emit runtime.expired
```

Это намеренно простой baseline для V01. Later versions могут добавить
per-node TTLs, budgets, pinning, quotas и resource-pressure eviction.

#### 8. Resurrect runtime

Resurrection - нормальный return path после expiry.

1. User открывает expired session или отправляет новый turn.
2. Core отправляет `ResumeRuntime(session_thread_id)` на Node.
3. Node запускает provider runtime в том же `workspace_path`.
4. Adapter восстанавливает provider session через `provider_resume_cursor` или
   `provider_session_id`.
5. Node отправляет `runtime.resuming`, затем `runtime.ready`.
6. Core обновляет projection, и UI может send/continue turn.

Если provider-native resume не сработал, Node отправляет
`runtime.resume_failed`. Core может предложить degraded resume: запустить новый
runtime в том же workspace и передать resume context package из session
summary, recent turns, open files, diff и constraints. UI должен явно показать,
что provider-native continuity не восстановлена.

#### 9. Stop runtime

Explicit Stop:

- останавливает provider process;
- помечает runtime как `stopped`;
- не удаляет session thread;
- не удаляет workspace files;
- записывает `runtime.stopped` with actor и reason.

После explicit Stop можно все еще предлагать Resume, если provider resume state
и workspace доступны. UI должен отличать "stopped by user" от "expired after no
steps".

### Core to Node command contract

V01 command set:

```text
StartRuntime {
  session_thread_id
  runtime_session_id
  workspace_binding
  provider = codex
}

ResumeRuntime {
  session_thread_id
  runtime_session_id
  workspace_binding
  provider_resume_ref
  resume_context optional
}

SendTurn {
  session_thread_id
  runtime_session_id
  turn_id
  message
}

SubmitApproval {
  request_id
  decision
  payload optional
}

InterruptRuntime {
  runtime_session_id
  reason
}

StopRuntime {
  runtime_session_id
  reason
}
```

Commands должны быть idempotent там, где это практично. Core должен передавать
ids, созданные на control-plane side, чтобы retry не создавал duplicate turns
или runtimes.

### Node to Core event contract

V01 events должны быть append-only и ordered per runtime session.

Minimum envelope:

```text
event_id
session_thread_id
runtime_session_id optional
turn_id optional
seq
kind
happened_at
node_id
payload
```

Minimum event kinds:

```text
runtime.starting
runtime.ready
runtime.running
runtime.blocked
runtime.resuming
runtime.expired
runtime.stopped
runtime.error
turn.started
turn.completed
turn.interrupted
turn.error
provider.output.delta
provider.message.completed
command.started
command.output.delta
command.completed
files.changed
diff.updated
approval.requested
approval.resolved
approval.expired
user_input.requested
user_input.resolved
```

Event log не является UI layout. UI строит projections из events: chat
messages, output panel, diff panel, trace panel, approval controls и status.

### Provider adapter contract

Codex adapter должен открыть небольшой internal interface:

```text
start(request) -> RuntimeHandle
resume(request) -> RuntimeHandle
send_turn(handle, turn_input) -> event stream
submit_approval(handle, request_id, decision)
interrupt(handle)
stop(handle)
snapshot(handle) optional
```

Adapter output является provider-specific на edge и нормализуется Node до того,
как Core его увидит. Если Codex дает structured app-server protocol, Node
должна использовать его. Если provider поддерживает только CLI resume, это
относится к later stateless/ephemeral strategy или degraded fallback.

### Diff and workspace observation

В V01 diff не требует full checkpoint/rollback.

Минимум:

- record baseline when session starts;
- compute changed files since baseline;
- optionally compute changed files per turn using filesystem timestamps и git
  diff snapshots;
- show current working tree diff in UI;
- attach file-change events to the active turn when possible.

Если exact attribution неясен, UI должен выбирать честную формулировку:
"changed during this session", а не делать вид, что каждая line связана с
точным agent action.

### Storage implications

Core storage должен поддерживать:

- session thread list and detail;
- latest runtime status projection;
- append-only event log;
- turns and messages;
- approval requests;
- provider resume reference;
- workspace binding;
- artifact/diff metadata.

Node storage должен поддерживать:

- mapping `runtime_session_id -> local process handle`;
- local runtime metadata for restart while Node process is alive;
- workspace path validation;
- provider local state location;
- local logs/output buffering until events reach Core.

Если Node перезапускается, она должна выполнить reconcile:

1. report live provider processes, которые она еще может identify;
2. mark unknown previous runtimes as `stale` или `expired`;
3. allow Core to request resurrection для selected session.

### Permissions and safety

V01 permission model может быть простой, но explicit:

- runtime scoped to one workspace binding;
- provider process runs with Node-local permissions, not Core database access;
- Core routes approvals и user-input decisions;
- Node records commands и file changes as events;
- dangerous tool permissions become visible approval requests when provider
  supports it;
- provider credentials stay on Node или provider-specific secure storage, not in
  browser state.

Agent не должен получать broad Cortex admin credentials только потому, что он
запущен внутри project session.

### Failure modes

| Failure | V01 behavior |
| --- | --- |
| Provider process exits | Node отправляет `runtime.error` или `runtime.expired` с reason; Core сохраняет thread readable. |
| Node disconnects | Core помечает runtime как `stale`; UI показывает last known state и reconnect/resume path. |
| Core restarts | Core загружает durable state; Node переподключается и отправляет runtime snapshot. |
| Resume cursor missing | UI предлагает degraded resume из saved thread/workspace context. |
| Workspace path missing | Runtime не может resume; UI показывает workspace error, historical thread остается readable. |
| Approval expires | Request становится expired, runtime может остановиться, trace фиксирует причину. |
| Event delivery duplicates | Core дедуплицирует по `event_id` и per-runtime `seq`. |

### V01 implementation checklist

- Core models: `SessionThread`, `WorkspaceBinding`, `RuntimeSession`, `Turn`,
  `RuntimeEvent` и approval requests.
- Core API: create/open session, send turn, approve, interrupt, stop и
  resume.
- Core event subscription endpoint для session thread updates.
- Node command handler для start/resume/send/approve/interrupt/stop.
- Codex provider adapter для live process/session runtime.
- Persist provider resume cursor/session id when available.
- Track `last_runtime_step_at` from meaningful runtime events.
- 24h no-steps expiry loop в Node.
- Runtime resurrection path для expired sessions.
- UI states: `ready`, `running`, `blocked`, `expired`, `resuming`, `stale`,
  `stopped` и `error`.
- Basic files/diff/output/trace panels from normalized events.

### Remaining architecture questions

- Which exact Codex runtime protocol should V01 adapter use first:
  app-server-style live protocol или CLI resume fallback?
- How much provider raw event data should Core persist for debugging, and how
  much should be normalized only?
- Do we need per-turn diff attribution in V01, or is session-level diff
  enough for the first implementation?
- Should `provider_resume_ref` live fully in Core, or should Core store only an
  opaque Node-owned reference?
- What is the exact degraded resume context package when provider-native resume
  is unavailable?
