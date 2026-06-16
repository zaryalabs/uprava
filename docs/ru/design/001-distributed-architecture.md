# A-001 Distributed Architecture

Статус: `working-position`

Этот документ фиксирует рабочую позицию по распределенной механике Cortex:
Core Backend является discovery/control plane, Node Daemon является data plane,
клиенты работают через Core, а связь с нодами строится вокруг outbound-модели,
регистрации демона и наблюдаемого heartbeat/status слоя.

## Vision

### Какую проблему решает механика

Cortex должен управлять агентами и рабочими окружениями, которые физически
могут находиться где угодно:

- локальный ноутбук пользователя;
- домашний компьютер без статического IP;
- сервер или VPS;
- devbox;
- cloud workspace;
- sandbox или будущий microVM host.

Клиенту нельзя требовать прямой доступ к каждой машине. Браузер, телефон или
CLI должны иметь один стабильный endpoint - Core Backend. Core хранит
глобальную модель системы, discovery, permissions, routing, event log and
trace. Node Daemon работает рядом с файлами, терминалом, процессами, локальными
секретами и agent runtime.

Главная задача распределенной механики - сделать эту модель надежной и понятной
при нестабильных адресах, NAT, sleep/resume ноутбуков, временных devbox,
локальных ограничениях безопасности и долгих agent sessions.

### Концептуально как реализуем

Базовая модель: **Node Daemon сам выходит в Core**.

Core не предполагает, что у ноды есть статический IP или доступный inbound
порт. Вместо этого daemon:

1. регистрируется в Core через короткий pairing-flow;
2. хранит node credential локально;
3. периодически отправляет зашифрованный и авторизованный heartbeat/status;
4. получает от Core желаемое состояние и, если нужно, запрос открыть
   двунаправленный control channel;
5. открывает outbound WebSocket/control stream в Core;
6. через этот канал получает команды и стримит events/output обратно.

Семантически это push-модель относительно Core: нода регулярно сообщает, что
она жива и чем располагает. Физически все базовые сетевые подключения начинает
Node Daemon, чтобы не зависеть от публичного адреса.

```text
Client
  |
  | HTTP/WebSocket to stable Core endpoint
  v
Core Backend
  ^
  | heartbeat every N seconds
  | optional outbound control channel
  |
Node Daemon
  |
  v
files / PTY / processes / agents / workspaces
```

Heartbeat не должен превращаться в скрытую шину выполнения команд. Его роль:
presence, health, capabilities snapshot, lightweight config sync and channel
negotiation. Конкретные операции над файлами, terminal, sessions and agents
должны идти через отдельный авторизованный control channel и попадать в event
log/trace.

### Безопасность как часть модели

Связь Core <-> Node должна быть защищена на двух уровнях:

- transport encryption: публичные и remote deployment используют HTTPS/WSS;
- application identity: каждый daemon после регистрации имеет node credential,
  которым авторизует heartbeat and control channel.

Pairing-code не является постоянным секретом. Это короткоживущий способ
доказать, что пользователь, который видит вывод daemon, действительно хочет
добавить эту ноду в Core. После approval Core выдает daemon постоянную
идентичность, которую можно rotate/revoke.

Рабочая позиция:

- daemon при первом запуске создает локальный installation identity;
- daemon отправляет enrollment request в Core и получает короткий pairing-code;
- пользователь вводит pairing-code в Core UI или CLI;
- Core привязывает enrollment к пользователю/проекту/organization scope;
- daemon получает high-entropy node token и становится registered node;
- Core хранит hash node token, а не сам token;
- все дальнейшие heartbeat/control requests проходят с node credential;
- Core может в любой момент revoke/suspend node credential.

Для Stage 1 оптимальный security setup:

- remote deployment: только `HTTPS/WSS`;
- local dev/single-user loopback: `HTTP/WS` допустим только на `127.0.0.1`;
- Node Daemon auth: random high-entropy bearer token;
- Core storage: token hash at rest;
- stream access: bearer token плюс short-lived scoped connection lease;
- browser/client auth: server-side session cookie, not browser JWT.

Ed25519 request signing, mTLS, per-message signatures and full PKI не нужны как
обязательный минимум Stage 1. Архитектурно протокол должен оставлять путь к
`auth_kind = token | signed_request | mtls`, key rotation and
hardware/keychain-backed storage, но первая реализация должна быть проще.

### Пользовательские сценарии

#### 1. Пользователь подключает личный ноутбук

Пользователь запускает Node Daemon на ноутбуке и указывает Core URL.
Daemon показывает код вида `CORTEX-ABCD-1234`. Пользователь открывает Core
Control Panel, выбирает "Add Node", вводит код, задает имя ноды и разрешенные
workspace roots. Через несколько секунд нода появляется в списке как online.

Пользователь не настраивает port forwarding, DNS, VPN или статический IP.

#### 2. Пользователь открывает session с телефона

Пользователь с телефона открывает Core UI и выбирает session, которая должна
работать на домашнем компьютере. Core видит последний heartbeat ноды.

Если control channel уже открыт, команды идут сразу. Если channel закрыт, Core
ставит для ноды `open_control_channel` request. На следующем heartbeat daemon
получает этот request, открывает outbound channel в Core, и UI подключается к
session stream через Core.

Worst-case latency равна heartbeat interval плюс время открытия channel. При
дефолте около 5 секунд это приемлемо для Stage 1. Для активной session channel
может оставаться открытым.

#### 3. Нода уходит в sleep или пропадает

Core не удаляет ноду и не считает ее сломанной мгновенно. Он переводит ее через
состояния `stale` and `offline` на основе пропущенных heartbeat. UI показывает:
последний heartbeat, последнюю известную health snapshot, активные sessions,
которые могли быть затронуты, и действия пользователя: wait, detach, stop when
node returns, revoke, inspect logs later.

#### 4. Серверная нода с постоянным размещением

Позже пользователь может пометить ноду как `static/reachable`. Это позволит
Core использовать более быстрый режим связи: держать постоянный control channel
или инициировать прямое подключение к известному endpoint.

Это оптимизация, а не базовая модель. Все permissions, event log, trace,
credential checks and protocol semantics остаются такими же, как у обычной
outbound-ноды.

### Agent-facing сценарии

Для Stage 1 worker agent не должен думать о distributed topology. Его рабочая
картина должна быть близка к обычному локальному запуску: есть project/workspace,
shell, файлы, tools and process environment в рамках выданного scope.

Нода остается инфраструктурной деталью для Core, UI, routing, permissions and
trace. Агенту не нужно знать node id, способ связи с Core, heartbeat, control
channel или то, что эта machine является одной из нод Cortex.

Практические следствия:

- агент видит project/workspace как локальную среду выполнения;
- доступные files, shell, tools and env уже отфильтрованы Node Daemon policy;
- agent tool calls не обходят Node Daemon и Core routing, но это не должно
  становиться частью обычного prompt/context агента;
- события agent work связываются с node/session/event log на уровне Core trace,
  а не как обязательная agent-facing модель;
- если нода degraded/offline, это сначала UI/Core/runtime состояние. Worker
  agent может получить обычную ошибку runtime/tool execution, а не подробности
  distributed инфраструктуры;
- позже internal Cortex agent or orchestrator может видеть node availability
  как системный объект, но это отдельная роль, не базовая картина worker agent.

### First release vs later

#### Stage 1

Для Developer Node Workbench достаточно:

- один Core URL;
- Node Daemon с outbound registration;
- short-lived pairing-code;
- registered node identity/credential;
- encrypted transport для non-local deployment;
- hashed node bearer token для daemon auth;
- server-side session cookies для client auth;
- configurable heartbeat interval, default около 5 секунд;
- heartbeat payload с минимальным health/capability snapshot;
- heartbeat response с lightweight config/version info and channel request;
- outbound WebSocket/control channel для active session work;
- Core UI для node list, add node, status, last heartbeat and revoke;
- SQLite-backed registry/event log на стороне Core;
- локальное хранение daemon config/credential на Node.

Stage 1 не требует:

- inbound-доступа к Node;
- direct client-to-node connections;
- сложного team RBAC;
- message queue;
- full mTLS rollout;
- signed request protocol;
- browser-stored JWT as default web auth;
- managed cloud node pool;
- advanced metrics store.

#### Later

Дальше можно добавить:

- mTLS или request signing как обязательный режим;
- credential rotation and device keychain integration;
- WebAuthn/TOTP/OIDC/SAML для users and teams;
- static/reachable node mode;
- relay mode or broker для сложных сетей;
- adaptive heartbeat interval;
- richer node telemetry;
- node pools and scheduling;
- multi-user/team permissions;
- managed Core/cloud deployment;
- stronger audit and security policy model.

## Architecture

### Основные сущности

#### Core Backend

Control plane and stable endpoint. Core owns:

- registered nodes;
- node enrollment state;
- node token hashes and credential metadata;
- latest heartbeat/status snapshot;
- capability registry for each node;
- desired node config/policy version;
- active connection leases;
- command routing;
- event log and trace metadata;
- user permissions around nodes and sessions.

Core не выполняет локальные shell/file/process операции вместо Node Daemon.

#### Node

Продуктовая сущность: зарегистрированное окружение, где может выполняться
работа. Node имеет identity, display name, owner/scope, status, capabilities,
policy and workspace bindings.

#### Node Daemon

Инфраструктурный процесс на Node. Он:

- проходит enrollment;
- хранит node token;
- отправляет heartbeat;
- открывает control channel;
- применяет локальную policy;
- управляет workspace/files/PTY/processes/agent sessions;
- стримит events/output/log summaries;
- может отказать в операции, если она нарушает локальные ограничения.

#### Enrollment

Короткоживущая регистрационная сессия, связывающая daemon, Core и пользователя.

Состояния:

```text
created -> pending_user_approval -> approved -> claimed
       \-> expired
       \-> rejected
```

`pairing_code` нужен только для approval. После `claimed` daemon получает
постоянный node credential.

#### Node Credential

Долгоживущая, но отзывная идентичность daemon. Для Stage 1 это random
high-entropy bearer token, который daemon хранит локально, а Core хранит только
как hash.

Важно, что:

- credential уникален для node installation;
- credential не показывается пользователю как pairing-code;
- credential не передается в URL/query string;
- credential не логируется;
- Core может revoke/suspend credential;
- daemon использует credential для HTTPS heartbeat and WSS control channel auth;
- credential rotation должна быть возможна без пересоздания Node;
- future hardened profiles могут заменить token на signed requests or mTLS.

#### Heartbeat

Периодический status signal от daemon в Core.

Он отвечает на вопросы:

- daemon жив?
- какая версия daemon/protocol?
- какие key capabilities доступны?
- какие sessions/processes активны?
- есть ли деградация?
- какая последняя локальная policy/config применена?
- открыт ли control channel?

Heartbeat не передает secrets, env values, полные списки файлов или большие
логи.

#### Control Channel

Двунаправленное соединение, которое daemon открывает outbound в Core. На Stage 1
лучший кандидат - WebSocket поверх TLS.

Через него идут:

- command requests от Core к Node Daemon;
- command responses;
- file/session/terminal streams;
- structured node/session events;
- backpressure/flow-control signals;
- keepalive and graceful close.

WebSocket здесь выбран не как универсальный API стиль, а как практичный
full-duplex transport для интерактивной работы: terminal stdin/stdout, resize,
stop, attach/detach, file/session streams and runtime control messages.

#### Connection Lease

Короткоживущий request от Core к daemon открыть или удерживать control channel.
Lease нужен, чтобы Core мог сказать "мне нужен канал для этой session", но
daemon все равно сам инициировал сетевое подключение.

Пример:

```text
lease_id
node_id
reason: session_attach | terminal_stream | file_watch | diagnostics
scope: project/session/workspace
expires_at
requested_by
```

### Границы ответственности

#### Core отвечает за

- node registry and discovery;
- enrollment approval;
- node credential lifecycle;
- permissions and policy decisions;
- routing user/client intent to nodes;
- desired node state;
- last known status;
- event log and trace metadata;
- UI-visible node lifecycle;
- revoke/suspend decisions.

#### Node Daemon отвечает за

- local process identity and credential storage;
- local capability probing;
- heartbeat emission;
- control channel initiation;
- local workspace/file/PTY/process/agent operations;
- local secrets and env;
- local resource limits;
- local policy enforcement;
- raw local logs where needed.

#### Client отвечает за

- human interaction;
- add node flow;
- node/session status visualization;
- attach/detach actions;
- review and command initiation through Core.

Client не хранит durable distributed state и не должен напрямую общаться с Node
в базовой модели.

### Communication model

#### 1. Enrollment

```text
Node Daemon                         Core Backend                      User
    |                                    |                            |
    | POST /node-enrollments             |                            |
    | public key / install id / label    |                            |
    |----------------------------------->|                            |
    | enrollment id + pairing code       |                            |
    |<-----------------------------------|                            |
    | print pairing code                 |                            |
    |                                                                 |
    |                                  User enters pairing code in UI |
    |                                    |<---------------------------|
    |                                    | approve enrollment          |
    | poll enrollment status             |                            |
    |----------------------------------->|                            |
    | node id + token/config             |                            |
    |<-----------------------------------|                            |
    | store token locally                |                            |
```

Pairing-code должен быть:

- коротким для ручного ввода;
- short-lived;
- single-use;
- rate-limited;
- не способным сам по себе выполнять операции;
- связанным с enrollment id and daemon public/install identity.

#### 2. Heartbeat loop

```text
every heartbeat_interval +/- jitter:

Node Daemon -> Core:
  authenticated heartbeat(status, capability_hash, health, channel_state)

Core -> Node Daemon:
  ack(server_time, config_version, desired_policy_version, optional channel lease)
```

Рекомендуемая Stage 1 настройка:

- default interval: `5s`;
- jitter: небольшой, чтобы много нод не били Core одновременно;
- stale threshold: примерно `2-3` пропущенных heartbeat;
- offline threshold: примерно `4-6` пропущенных heartbeat;
- active session может держать постоянный channel, чтобы не ждать interval.

Точные значения должны быть конфигурируемыми.

#### 3. On-demand control channel

```text
Client -> Core:
  attach to session / open terminal / inspect files

Core:
  if no active channel:
    create connection lease for node

Node Daemon -> Core:
  next heartbeat

Core -> Node Daemon:
  heartbeat ack includes lease_id + requested channel scope

Node Daemon -> Core:
  open outbound WebSocket /node-control?lease_id=...

Core:
  bind channel to node/session routing

Client:
  receives stream through Core
```

Для active sessions Core может просить daemon удерживать channel до idle timeout.
Для idle нод достаточно heartbeat loop.

#### 4. Static/reachable mode later

Static mode не должен создавать вторую архитектуру. Он меняет только transport
optimization:

- Core может знать endpoint ноды;
- Core может инициировать probe/direct channel;
- daemon может держать always-on channel;
- latency ниже;
- auth, permissions, event contracts and trace остаются теми же.

### Transport and protocol layering

Cortex не должен ограничиваться одним интерфейсом для всех типов связи.
Management plane, event feed and interactive runtime имеют разные требования.

Рабочая позиция для Stage 1:

```text
Node Daemon -> Core:
  HTTPS JSON API   enrollment, heartbeat, capabilities, config/policy sync
  WSS control      active bidirectional node/session/control channel

Client -> Core:
  HTTPS JSON API   CRUD, commands, decisions, snapshots
  SSE              status, timeline, trace and event feed
  WSS              interactive session/terminal attach
```

#### Node Daemon -> Core

System-level operations should use HTTPS JSON APIs:

- enrollment request;
- pairing status polling;
- heartbeat;
- capability snapshot upload;
- config/policy fetch;
- diagnostics snapshot upload;
- credential rotation/revoke acknowledgement.

These calls are request/response, easy to retry, easy to log, and do not require
a long-lived channel.

Interactive work should use outbound WebSocket:

- persistent session command routing;
- PTY output and stdin;
- terminal resize;
- process stop/interrupt;
- file watch/change streams;
- agent session event streams;
- backpressure and flow-control messages.

WebSocket is the Stage 1 default because it is full-duplex, browser-compatible
on the Core/client side, straightforward in Axum/Tokio, and works with the
outbound connection model. gRPC bidirectional streaming remains a possible later
transport for Core <-> Node, but it should not be required for Stage 1.

#### Client -> Core

Client management should use HTTPS JSON APIs:

- list nodes/sessions/projects;
- add/revoke node;
- start/stop session;
- submit user decisions;
- fetch snapshots and artifacts.

Client passive live updates should use SSE:

- node status changes;
- event timeline;
- trace feed;
- artifact/session status updates.

SSE is enough when data moves Core -> Client only. It has a simpler browser
model than WebSocket, supports reconnect well, and maps cleanly to ordered event
feeds.

Client interactive attach should use WebSocket:

- terminal input/output;
- session live control;
- stdin/resize/interrupt;
- low-latency bidirectional UI interactions.

#### Stage 1 security by channel

| Channel | Transport | Auth | Extra protection |
| --- | --- | --- | --- |
| Node -> Core system API | `HTTPS` remote, `HTTP` only on `127.0.0.1` local dev | `Authorization: Bearer <node_token>` | Core stores token hash, rate limits auth failures, logs significant auth events |
| Node -> Core control stream | `WSS` remote, `WS` only on `127.0.0.1` local dev | node bearer token | short-lived scoped connection lease, no token in query string, channel audit events |
| Client -> Core API | `HTTPS` remote, `HTTP` only on `127.0.0.1` local dev | server-side session cookie | `HttpOnly`, `Secure`, `SameSite`, CSRF protection for mutations |
| Client -> Core SSE | `HTTPS` remote, `HTTP` only on `127.0.0.1` local dev | server-side session cookie | Origin check, reconnect/event authorization |
| Client -> Core WebSocket | `WSS` remote, `WS` only on `127.0.0.1` local dev | server-side session cookie | Origin check, per-session authorization, attach/detach audit events |

#### Message protocol over streams

WebSocket should not become an ad hoc endpoint-specific API. It should carry a
small versioned message envelope that can be reused across transports later.

Conceptual envelope:

```json
{
  "type": "session.pty.chunk",
  "id": "msg_123",
  "correlation_id": "cmd_456",
  "stream_id": "pty_1",
  "seq": 42,
  "sent_at": "2026-06-16T10:20:30Z",
  "payload": {}
}
```

Envelope requirements:

- `type` identifies domain message kind;
- `id` identifies the message;
- `correlation_id` links request/response/event chains;
- `stream_id` separates terminal/file/session streams over one channel;
- `seq` supports ordering and gap detection per stream;
- `payload` is typed by message kind;
- protocol version is negotiated at channel open.

Initial message families:

```text
node.channel.opened
node.channel.keepalive
node.channel.error
node.channel.close

command.request
command.accepted
command.output
command.completed
command.failed
command.cancel

session.attach
session.detach
session.event

session.pty.open
session.pty.input
session.pty.chunk
session.pty.resize
session.pty.close

file.read
file.chunk
file.watch
file.changed

flow.pause
flow.resume
flow.ack
```

This keeps domain protocol separate from transport choice. Later, the same
message families can move to gRPC streaming, WebTransport, a relay, or another
broker if the deployment model requires it.

#### Authentication and channel leases

Daemon heartbeat and management calls authenticate with node bearer token over
HTTPS. Core stores only token hash and compares presented tokens using a
constant-time check.

Control channel opening should additionally use a short-lived connection lease:

- Core creates lease when a session/terminal/file stream needs a node channel;
- lease id/token is returned to daemon in heartbeat response;
- daemon opens `wss://core/...` using normal auth plus lease proof;
- lease expires quickly and is scoped to node/session/reason;
- long-lived node credential should not be placed in query string.

Client WebSocket/SSE connections authenticate with user session auth, not node
credentials.

Stage 1 browser auth should use Core-managed server-side sessions:

- session cookie is `HttpOnly`, `Secure` on HTTPS, and `SameSite`;
- mutation APIs use CSRF protection;
- SSE/WSS check authenticated user session and request Origin;
- login and pairing endpoints are rate-limited;
- passwords, if local login exists, are stored with Argon2id.

CLI/API clients can later use personal access tokens, but browser JWT should
not be the default web auth mechanism.

#### Why not one universal protocol

One universal protocol would blur different operational needs:

- heartbeat should be cheap, retryable, and mostly stateless;
- event feed should be ordered, replayable, and easy for browsers;
- terminal/session work needs low-latency full-duplex streaming;
- management commands need clear request/response semantics and audit records.

The stable contract should be the domain messages, events, permissions and trace
semantics. Transport is an implementation layer that can vary by use case.

### Heartbeat payload

Концептуальная форма:

```json
{
  "node_id": "node_123",
  "daemon_version": "0.1.0",
  "protocol_version": "node-protocol-v1",
  "sequence": 1042,
  "sent_at": "2026-06-16T10:20:30Z",
  "uptime_seconds": 8640,
  "capability_hash": "sha256:...",
  "health": {
    "status": "ok",
    "cpu_load": 0.42,
    "memory_used_ratio": 0.61,
    "disk_free_bytes": 120034123776,
    "battery_state": "charging"
  },
  "runtime": {
    "active_sessions": 2,
    "running_processes": 5,
    "open_pty": 1
  },
  "connection": {
    "control_channel": "closed",
    "open_leases": []
  },
  "policy": {
    "applied_policy_version": 7
  }
}
```

Пример response:

```json
{
  "ack": true,
  "server_time": "2026-06-16T10:20:31Z",
  "desired_policy_version": 7,
  "desired_config_version": 3,
  "connection_request": {
    "lease_id": "lease_456",
    "reason": "session_attach",
    "session_id": "session_789",
    "expires_at": "2026-06-16T10:21:31Z"
  }
}
```

Поля являются design sketch, не финальным API contract.

### Node status model

Node status должен быть отдельным от session status. Нода может быть online, но
конкретная agent session может быть stopped. Или нода может быть offline, но
Core все еще хранит last known session state.

Минимальная модель:

```text
unregistered
enrolling
online
stale
offline
degraded
suspended
revoked
```

Дополнительные flags:

```text
control_channel: closed | requested | connecting | open | draining | error
capabilities: current | changed | unknown
policy: current | update_pending | rejected
```

`degraded` не заменяет `online/offline`. Это health flag: нода отвечает, но не
может полноценно работать из-за disk pressure, missing tool, old daemon version,
credential issue, denied workspace, failed self-check или другой причины.

### Capability model

Heartbeat несет `capability_hash`, а полная capability snapshot передается при:

- enrollment;
- daemon version change;
- capability hash change;
- explicit Core request;
- periodic refresh.

Capabilities могут включать:

- supported protocol versions;
- OS/arch;
- workspace root policy;
- available shells;
- PTY support;
- file operations support;
- installed/default agents;
- known local tools;
- git availability;
- container/sandbox availability;
- resource limits;
- supported artifact/output streams.

Core использует capabilities для UI, routing, permissions and session launch
constraints. Node Daemon все равно делает local enforcement перед выполнением.

### Events and trace

Распределенная механика должна быть visible and traceable. Core пишет events:

```text
node.enrollment_created
node.enrollment_approved
node.registered
node.heartbeat_received
node.status_changed
node.capabilities_changed
node.connection_requested
node.connection_opened
node.connection_closed
node.policy_updated
node.command_routed
node.command_rejected
node.suspended
node.revoked
```

Не каждый heartbeat должен становиться шумным timeline event в UI. Core может
хранить latest status and rolling history, а в event log выводить только
значимые transitions: online -> stale, stale -> offline, capability changed,
control channel error, command rejected, policy mismatch.

Trace for agent work должен ссылаться на:

- node id;
- session id;
- command/tool call id;
- workspace id/path alias;
- connection/channel id where relevant;
- causality links to command output, file changes and artifacts.

### Storage implications

#### Core storage

Core должен хранить:

- nodes;
- node enrollments;
- node token hashes;
- node display names and ownership scope;
- node status latest snapshot;
- heartbeat sequence/last_seen_at;
- capability snapshots;
- node policy/config versions;
- active connection leases;
- control channel metadata;
- significant node events;
- routing/audit records for commands.

Raw high-frequency metrics можно сначала не хранить или хранить коротким rolling
window. Для Stage 1 важнее latest status and meaningful transitions.

#### Node storage

Node Daemon должен хранить:

- core URL;
- node id;
- node token;
- local daemon config;
- local policy cache;
- workspace bindings;
- local session/process metadata;
- raw logs/output where needed;
- pending events buffer for reconnect.

Node token должен храниться в protected local config for Stage 1. Later можно
перейти на OS keychain/credential manager or keypair-backed identity.

### Permissions and policy

Минимальные permission decisions:

- кто может register node;
- кто может approve enrollment;
- кто может view node;
- кто может attach to session on node;
- кто может open terminal/PTY;
- кто может read files;
- кто может write/apply patches;
- кто может start/stop agent session;
- кто может revoke/suspend node.

Core принимает глобальное решение. Node Daemon применяет локальное решение.

Пример: Core разрешил пользователю открыть terminal, но daemon должен проверить,
что requested workspace входит в allowed roots, session принадлежит этой ноде,
policy version применена, а локальные safety limits не нарушены.

Отказ daemon должен быть structured event, а не только текстовая ошибка:

```text
node.command_rejected
reason: workspace_not_allowed | policy_mismatch | capability_missing | resource_limit | local_lockdown
```

### Failure modes

#### Core недоступен

Daemon должен:

- продолжать локально running sessions where safe;
- buffer important events within limits;
- retry heartbeat with backoff;
- show local diagnostic status;
- not accept new remote commands without Core.

#### Node offline/sleep

Core должен:

- mark stale/offline based on missed heartbeat;
- keep last known status;
- avoid sending commands into a void;
- show waiting state in UI;
- resume routing when node returns.

#### Pairing-code украден или введен не тем пользователем

Mitigations:

- short TTL;
- single-use;
- rate limiting;
- UI confirmation with daemon-provided label/fingerprint;
- ability to reject/revoke enrollment;
- pairing-code grants enrollment only, not runtime access.

#### Credential stolen

Mitigations:

- revoke credential from Core;
- rotate credential;
- store only token hash in Core;
- do not log token or pass it in query string;
- bind credential to node install identity later where useful;
- audit unusual heartbeat/source patterns;
- protected local config for Stage 1;
- future keychain, mTLS or request signing.

#### Duplicate daemon / cloned disk

Core should detect conflicting heartbeats:

- same node id from different install fingerprint;
- sequence rollback;
- simultaneous control channels where policy allows only one;
- changed device fingerprint.

Resolution can be `suspended_pending_review` until user confirms.

#### Version mismatch

Heartbeat carries daemon/protocol version. Core can respond:

```text
accepted
accepted_with_warning
upgrade_required
protocol_unsupported
```

UI should show degraded status and required action.

#### Backpressure

Control channel must support flow control. Terminal/file streams can be large.
Core should avoid unbounded buffering, and Node Daemon should receive explicit
slow/stop signals.

### UI consequences

Stage 1 UI should include a Nodes surface:

- node list/table;
- status: online, stale, offline, degraded, suspended;
- last heartbeat time;
- active sessions;
- daemon version;
- selected capabilities;
- current control channel state;
- add node flow with pairing-code;
- revoke/suspend action;
- diagnostics panel for the latest structured error.

When a user opens a session on an idle node and no control channel exists, UI
should show a clear transient state:

```text
Waiting for node heartbeat...
Requesting secure control channel...
Connected
```

This avoids the feeling that the app is randomly slow. The delay is a visible
consequence of dynamic-address outbound communication.

### Tests/evals/checklist

Minimum design acceptance checklist:

- A node behind NAT can register without inbound ports.
- A user can pair daemon by entering a short code in Core.
- Pairing-code expiry and rejection work.
- Heartbeat auth failure is rejected and logged.
- Missed heartbeat moves node to stale/offline.
- Core can request a control channel through heartbeat response.
- Daemon opens outbound channel and Core routes a session command through it.
- UI distinguishes node status from session status.
- Node can reject a command based on local policy.
- Revoked node credential cannot heartbeat or open channel.
- Core stores node token hash, not plaintext token.
- Node token is not accepted in query string for WSS control channel.
- Capability change is visible to Core and UI.
- Active session can keep channel open to avoid heartbeat latency.
- Idle node can close channel and remain discoverable through heartbeat.

### Open questions

- Which exact token hash format should Core use for node tokens?
- How transport-neutral should the Stage 1 stream envelope be while WebSocket is
  the initial control-channel transport?
- Which health fields are safe and useful by default without leaking local
  privacy?
- Should heartbeat interval be fixed per daemon, centrally configured by Core,
  or adaptive by node/session state?
- How much rolling heartbeat history should Core store before a metrics backend
  exists?
- How visibly should UI mark local loopback `HTTP/WS` deployment profile?
- What exact UX should confirm that the pairing-code shown by daemon matches
  the enrollment being approved in Core?
