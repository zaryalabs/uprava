# Контракты Agent Tooling и Tool Registry v1

Статус: `contract-v1`

Целевой срез: `0.2.11`

Реализация Core baseline: Epic 1 завершён 2026-07-19. Локальная реализация
Node/ToolHive runtime эпика 2 добавлена 2026-07-19. Контракты обслуживаются
SQLite migration `12`, application services Registry/Search/Inspect/Execute,
Uprava MCP Streamable HTTP endpoint `/mcp` на pinned `rmcp 2.2.0` и typed
durable `CommandKind::Tooling` path до Node. Доступ к MCP требует short-lived
session lease; Web read routes для definitions, observed capabilities,
dependency state, availability и calls используют те же application services.
Реальный Linear OAuth acceptance остаётся заблокирован внешним gate из spike;
production path не подменяет его mock credential или direct upstream fallback.

Этот документ фиксирует общий язык Core, Node, Uprava MCP и Web до начала
реализации отдельных эпиков. Канонические Rust-типы находятся в
`crates/uprava-protocol/src/tooling.rs`; документ задаёт их семантику,
совместимость и security-инварианты.

## Короткое решение

- Core остаётся authority для definitions, policy, scope, routing и trace.
- MCP revision: стабильная `2025-11-25`.
- Rust SDK для Uprava MCP: `rmcp = 2.2.0`, exact pin `=2.2.0` при добавлении
  runtime dependency.
- Node external runtime baseline: ToolHive CLI `0.40.0`.
- Model-visible surface по умолчанию содержит только `search_tools`,
  `inspect_tool`, `execute_tool`.
- Shared wire payload version: `TOOLING_CONTRACT_VERSION_V1 = 1`.
- Definition существует независимо от availability.
- Observed capability не является managed tool.

## Dependency decision record

### MCP SDK

Выбран официальный Rust SDK
[`rmcp` 2.2.0](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v2.2.0),
Apache-2.0.

Причины выбора:

- server и client roles;
- Streamable HTTP server/client transports;
- сохраняемая JSON Schema fidelity через `schemars` и явные schemas;
- cancellation, progress и notifications;
- `tools/list_changed`;
- OAuth primitives и возможность поставить Core auth middleware перед MCP;
- исправления по conformance для MCP `2025-11-25` в `2.2.0`;
- upstream совместим с используемыми Axum `0.8`, Tokio и Serde.

Планируемый feature set Core:

```toml
rmcp = { version = "=2.2.0", default-features = false, features = [
  "server",
  "macros",
  "schemars",
  "transport-streamable-http-server",
] }
```

`auth` не является заменой product lease contract. Session-scoped lease
проверяется Core application/auth boundary до MCP handler. SDK dependency
добавляется в Cargo только вместе с реальным endpoint, чтобы foundation gate не
оставлял unused runtime dependency.

### MCP revision

Поддерживаемая revision — `2025-11-25`. Revision `2026-07-28`, которую
ToolHive `0.40.0` уже умеет классифицировать до даты её стабильного выпуска, в
срез не принимается. Переход требует отдельного compatibility review.

### ToolHive

Baseline: ToolHive CLI `0.40.0`, commit
`505df835ed73790bb9be7db7944ec772dc136a0e`, Apache-2.0.

Release artifacts существуют для macOS amd64/arm64, Linux amd64/arm64 и
Windows amd64/arm64. Локально проверен macOS arm64 artifact с SHA-256:

```text
77dbd6f657fa2ad9676b284beab8630e11f36e9014045993c0b7e6db3cd62dbb
```

Результат конкретного Linear spike и exact commands находятся во временной
записи
[`0.2.11-toolhive-linear-spike.md`](../tmp-plans/0.2.11-toolhive-linear-spike.md).

## Identity и namespace

| Тип | Stable identity | Owner | Lifecycle |
| --- | --- | --- | --- |
| `ToolId` | `uprava.<domain>.<verb>` для native; `<integration>.<normalized_upstream_name>` для external | Core | Не переиспользуется для другого смысла |
| `ToolSourceId` | стабильный logical source, например `uprava-native`, `linear-remote-mcp` | Core | Живёт дольше connection/runtime |
| `IntegrationId` | UUID/opaque id одной product connection | Core | Создание, connect/reconnect, disconnect, delete позже |
| `McpDependencyInstanceId` | UUID/opaque id одного desired/actual runtime instance | Core создаёт, Node исполняет | Desired state сохраняется при Node restart |
| `ToolCallId` | UUID/opaque id до policy/routing | Core | Не переиспользуется; terminal state обязателен |
| `McpAccessLeaseId` | UUID/opaque id одной session lease | Core | Expiry, rotation, explicit revocation |

Wire representation всех identifiers — JSON string. Клиенты считают значение
opaque и не извлекают из него policy или routing.

## Definition и schema versioning

`ToolDefinition` содержит source identity, agent-visible metadata, input/output
schemas, risk, permissions, execution и redaction policy.

Правила:

1. `version` начинается с `1` и увеличивается при изменении agent-visible
   semantics, schema, risk, permissions, approval, routing или redaction.
2. `schema_hash` имеет форму `sha256:<lowercase hex>`.
3. Hash считается над объектом `{input_schema, output_schema}` после
   рекурсивной сортировки JSON object keys. Порядок arrays значим.
4. Availability, runtime health и connection state не меняют definition
   version/hash.
5. Description-only correction увеличивает `version`, но может сохранить
   `schema_hash`.
6. Definition state: `active | deprecated | disabled`.
7. Secret values и credential refs запрещены в definition и schema metadata.

`ToolAvailability` вычисляется заново для полного `ToolScope` и содержит
конкретные `state`, `reason`, `schema_hash`, `policy_version` и `observed_at`.
Она не является authorization grant.

`ObservedCapability` содержит только safe Node inventory. У неё нет `ToolId`,
input/output schema или managed execution route.

## Lifecycle vocabularies

### Availability

```text
available | unavailable | degraded | approval_required
```

Unavailable reason:

```text
node_offline | capability_missing | dependency_missing |
dependency_unhealthy | not_authenticated | permission_denied |
policy_blocked | project_not_enabled | session_not_enabled |
schema_changed | backend_unreachable | toolhive_missing
```

### MCP dependency actual state

```text
toolhive_missing | missing_auth | installing | starting | running |
degraded | failed | stopped
```

`toolhive_missing` — обязательный безопасный fallback: definition и desired
state сохраняются, effective availability становится false, execution не
пытается вызвать shell fallback или прямой upstream.

### Tool call

```text
requested | authorized | approval_required | started |
completed | failed | denied | cancelled | timed_out
```

Terminal states:

```text
completed | failed | denied | cancelled | timed_out
```

После создания `ToolCallId` любой путь обязан завершиться ровно одним terminal
state. Retry создаёт новый dispatch attempt внутри того же idempotent call
contract, а не вторую логическую запись вызова.

## Progressive discovery

### `search_tools`

Request: `SearchToolsRequest`.

```text
scope: ToolScope
query: string
filters: source_kinds[], risk_levels[], availability_states[]
cursor: opaque string | null
limit: integer | null
```

- default limit: `10`;
- maximum limit: `25`;
- Core отклоняет negative/zero и значения выше maximum;
- cursor opaque, scope/filter/query-bound и истекающий;
- visibility/permission filter применяется до ranking и count;
- denied tools не влияют на visible count или pagination;
- result содержит identity, name, short description, source, risk,
  availability, unavailable reason и schema hash;
- input/output schemas и permission diagnostics в Search запрещены.

### `inspect_tool`

Request: `InspectToolRequest { scope, tool_id }`.

Response: ровно один `InspectToolResponse { definition, availability,
invocation_mode }`.

Inspect выполняет fresh visibility check и возвращает current definition.
Unavailable definition остаётся inspectable только если actor может видеть сам
tool; reason ограничен безопасным vocabulary. Inspect не выдаёт capability
grant и не фиксирует policy для будущего Execute.

`invocation_mode`:

- `stable_execute_tool` — обязательный v1 path;
- `dynamic_mount_optional` — provider optimization после Inspect, не
  обязательный для корректности.

### `execute_tool`

Request: `ExecuteToolRequest { scope, tool_id, arguments }`.

Порядок обязательных проверок:

```text
lease and scope
-> current definition and schema
-> current visibility/permission
-> approval policy
-> current effective availability
-> request/result limits
-> create/continue trace and route
```

Response: `ExecuteToolResponse { tool_call_id, state, result, error }`.

- `completed` требует `result != null` и `error == null`;
- terminal failure требует `error != null` и `result == null`;
- non-terminal accepted state не содержит raw result;
- result limit: `1 MiB`, после чего используется bounded summary/artifact ref
  либо `result_too_large`;
- result и error details уже redacted до transport serialization.

Error codes:

```text
invalid_arguments | permission_denied | approval_required | unavailable |
schema_changed | rate_limited | request_too_large | result_too_large |
timeout | cancelled | backend_failed | toolhive_missing |
not_authenticated | scope_mismatch | lease_expired | lease_revoked
```

### Schema changes и `tools/list_changed`

1. Source refresh строит полный candidate set и hashes вне active index.
2. Core atomically swaps definitions/index only after validation.
3. Existing Inspect snapshot не разрешает Execute со старой schema.
4. Execute возвращает `schema_changed` и current hash без раскрытия denied
   metadata.
5. Uprava MCP отправляет `notifications/tools/list_changed` только при
   изменении трёх meta-tools. Registry content changes не раздувают MCP
   `tools/list`; session может получить product-level availability event.
6. Optional dynamic mount должен быть снят/обновлён provider adapter; stable
   `execute_tool` остаётся fallback.

## Core-to-Node contract

Outer durable `CommandEnvelope`/`EventEnvelope` остаётся authority для ids,
actor, target, correlation, causality и ordering. `ToolingCommandV1` и
`ToolingEventV1` — versioned payloads, которые Epic 1 встраивает в этот путь.
Extension payload для них запрещён.

Commands:

- `execute_external_tool`;
- `cancel_tool_call`;
- `update_dependency_desired_state`.

Events:

- `dependency_actual_state_reported`;
- `tool_definitions_discovered`;
- `tool_call_started`;
- `tool_call_completed`;
- `tool_call_failed`;
- `tool_call_denied`;
- `tool_availability_changed`.

`contract_version = 1` обязателен. Unknown major payload version отклоняется с
typed compatibility error; он не десериализуется как Extension. Command
duplicate определяется outer `command_id`, tool execution duplicate —
`tool_call_id`.

### Реализованный Node runtime baseline

- heartbeat репортит typed observed inventory для ToolHive, `git`, `gh` и
  `glab`; auth status ограничен значениями `authenticated | not_authenticated`;
- Core повторно отправляет desired dependency snapshot после Node reconnect, а
  Node сохраняет его в private local state;
- ToolHive CLI boundary принимает только pinned Linear upstream и безопасные
  workload/namespace identifiers;
- local MCP bridge выполняет `initialize`, `notifications/initialized`,
  `tools/list` и `tools/call`, проверяет current schema hash перед вызовом и
  ограничивает metadata, schema, process output и MCP result;
- timeout и cancel используют общий Node cancellation registry, duplicate outer
  command возвращает сохранённый terminal result;
- Core проецирует actual status и discovered definitions, вычисляет
  Node/auth/dependency-specific availability и восстанавливает terminal
  tool-call state из durable command result при потере process-local waiter;
- disconnect немедленно закрывает Core availability, увеличивает credential
  generation и отправляет disabled desired state для ToolHive cleanup.

## Authentication и policy

### MCP access lease

Lease — short-lived Core-signed bearer credential вне prompt/transcript.
Claims представлены `McpAccessLeaseClaims`:

- audience строго `uprava:mcp`;
- actor;
- session;
- project optional и placement required;
- Node placement;
- issued/expiry;
- credential version;
- unique lease id.

Baseline TTL: 10 минут. Provider adapter обновляет lease до expiry, не меняя
model context. Core проверяет signature, audience, expiry, credential version,
revocation, session state и exact scope на каждом MCP request.

Revocation происходит при session stop, actor/session permission change,
credential rotation, placement move и явном revoke. Browser cookie не является
MCP credential. Foreign session/project/node scope возвращает
`scope_mismatch` без existence disclosure.

### Policy decision

```text
allow | deny | require_approval
```

Search применяет только visibility portion. Inspect вычисляет fresh decision,
но не сохраняет grant. Execute повторяет full decision. Approval связывается с
actor, scope, tool id, schema hash, normalized argument hash, policy version и
expiry; любое изменение делает approval stale.

### OAuth connection

- connect начинается authenticated Web mutation с CSRF protection;
- callback state random, one-time, TTL 10 минут и связан с actor,
  integration, project, Node и exact redirect URI;
- callback state и authorization code не пишутся в обычные logs/events;
- Core хранит connection metadata, не token value;
- credential material живёт в trusted Node/ToolHive secret boundary;
- reconnect заменяет credential generation atomically;
- disconnect сначала немедленно делает Core availability false, затем
  best-effort отзывает upstream grant, удаляет local credential и переводит
  dependency в `missing_auth | stopped`;
- повторный disconnect idempotent;
- failure remote revocation отображается отдельным безопасным diagnostic, но
  Core connection не возвращается в available.

## Redaction и audit

- Secret values, OAuth codes/tokens, PKCE verifier, cookie, bearer lease и
  credential refs запрещены в model output, Web DTO, registry, events и logs.
- `ToolRedactionPolicy` использует JSON Pointer redact rules и bounded
  summaries.
- До persistence сохраняются hashes, sizes, refs и redacted summary.
- Raw arguments/results хранятся только по отдельной будущей retention policy;
  v1 не требует их persistence.
- Upstream tool metadata считается untrusted: names, descriptions и schemas
  проходят size limits, JSON Schema validation и normalization.
- Audit обязателен для deny, approval, mutation policy и credential lifecycle.

## API/Web contract

Web работает только с Core. Зафиксированные read routes:

```text
GET /api/v1/tool-definitions
GET /api/v1/tool-definitions/{tool_id}
GET /api/v1/tool-availability
GET /api/v1/nodes/{node_id}/observed-capabilities
GET /api/v1/integrations
GET /api/v1/mcp-dependencies
GET /api/v1/tool-calls
GET /api/v1/tool-calls/{tool_call_id}
```

Mutations:

```text
POST /api/v1/integrations/{integration_id}/connect
POST /api/v1/integrations/{integration_id}/reconnect
POST /api/v1/integrations/{integration_id}/disconnect
```

Canonical DTOs: `ToolDefinition`, `ToolAvailability`, `ObservedCapability`,
`IntegrationConnectionSummary`, `McpDependencyStatus`, `ToolCallSummary`,
`ToolCallDetail`, `IntegrationConnectRequest/Response`,
`IntegrationDisconnectRequest/Response` и list response types.

Rust fixture `tooling_contract` генерируется через
`crates/uprava-protocol/examples/web_fixtures.rs`. Web types и strict Zod
validators находятся в `apps/web/src/shared/protocol`; `protocol-check` ловит
literal drift и fixture drift.

## Compatibility

- В пределах payload v1 допускаются только optional additive fields с
  documented default.
- Required field removal/rename, enum semantic change и discriminator change
  требуют нового payload version.
- Strict Web enum additions проходят осознанный Rust/Web update через
  `protocol-check`.
- Unknown tooling payload version отклоняется; silent downgrade запрещён.
- Pre-1.0 HTTP/MCP contracts могут меняться только вместе с canonical fixtures,
  migration note и consumer update.
- Definition version/schema hash решают tool-level drift; protocol version
  решает envelope-level drift.

## Threat checklist этапа 0

- [x] Permission filtering происходит до Search ranking/count.
- [x] Inspect не создаёт grant; Execute повторяет checks.
- [x] Lease связан с actor/session/project/placement/Node/audience/expiry.
- [x] Есть rotation/revocation contract.
- [x] OAuth state one-time, bounded и scope-bound.
- [x] Disconnect сначала закрывает Core availability.
- [x] Secret values запрещены во всех shared DTOs и fixtures.
- [x] Argument/result redaction выполняется до persistence/transport.
- [x] Metadata/schema upstream считаются untrusted и bounded.
- [x] Managed tools и observed capabilities разделены типами.
- [x] `toolhive_missing` не приводит к direct upstream fallback.
- [x] Tool call имеет terminal-state invariant и correlation refs.
- [x] Core-to-Node использует typed versioned payload, не Extension.
- [x] MCP stable revision закреплена; future revision не принимается молча.
- [ ] Linear OAuth, discovery, read-only call и disconnect подтверждены в
  разрешённом test workspace — текущий внешний gate заблокирован политикой
  доступа к Linear, см. spike note.
