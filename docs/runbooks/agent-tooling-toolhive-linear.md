# Эксплуатация Agent Tooling, ToolHive и Linear

Статус: `local-runtime-ready-external-acceptance-blocked`

Этот runbook описывает локально реализованный baseline `0.2.11`: Uprava MCP,
Tool Registry, выдачу MCP-доступа Codex-сессиям, Node Tool Runtime и
человеческую поверхность `/settings/tooling`.

Реальный Linear OAuth consent, `tools/list`, read-only call и remote revoke пока
не прошли приёмку: действующая browser policy запрещает `linear.app`. Нельзя
обходить этот gate ручным token, другим браузером или фиктивным состоянием
`connected`.

## Проверка компонентов

1. Запустите Core, Node и Web обычным локальным профилем:

   ```sh
   make core-r
   make node-r
   make web-r
   ```

2. Откройте `/settings/tooling`.
3. Выберите workspace и session. Экран должен отдельно показать:

   - configured integration и её `desired`/`auth` state;
   - actual ToolHive state на Node;
   - effective availability managed tools в выбранной session;
   - observed `git`, `gh`, `glab` без кнопки proxy execution;
   - recent redacted tool calls и ссылки на session trace/result refs.

4. Проверьте установленный runtime:

   ```sh
   thv version
   ```

Поддержанный compatibility baseline — ToolHive CLI `0.40.0`. Точные временные
команды и checksum spike находятся в
[`0.2.11-toolhive-linear-spike.md`](../tmp-plans/0.2.11-toolhive-linear-spike.md).
Production Node использует `UPRAVA_TOOLHIVE_BINARY`, если binary не находится
через обычный `PATH` daemon user.

## Codex и Uprava MCP

Перед каждым новым `SendTurn` Node получает у Core краткоживущий
session-scoped lease через authenticated Node transport. Lease:

- не сохраняется в durable command;
- не добавляется в prompt, transcript или process arguments;
- передаётся Codex только через `UPRAVA_MCP_ACCESS_TOKEN`;
- ротирует предыдущий active lease этой session;
- отзывается при stop session и ротации Core credential.

Codex получает один Streamable HTTP MCP server `uprava`. Постоянная
model-visible surface этого server состоит только из `search_tools`,
`inspect_tool`, `execute_tool`. Полные schemas появляются только после Inspect.
Default Node profile задаёт `UPRAVA_CODEX_IGNORE_USER_CONFIG=true`, чтобы
managed session не наследовала произвольные пользовательские MCP mounts;
Codex authentication при этом продолжает использовать `CODEX_HOME`.

Если Core не может выдать lease, Node не запускает turn без tooling: command
завершается с `provider.mcp_access_unavailable`. Следующий явный turn повторяет
получение доступа и создаёт новый lease.

## Подключение и отключение Linear

До завершения внешнего gate кнопки Connect/Reconnect намеренно disabled. Это
не означает отсутствие backend: desired/actual reconciliation, discovery,
execution bridge и safe disconnect уже реализованы.

Disconnect доступен для существующей connection и выполняет две операции
атомарно в Core:

1. переводит desired state в `disabled` и auth state в `disconnected`;
2. немедленно закрывает effective availability и отправляет новый desired
   snapshot на Node.

UI отдельно сообщает, что `remote_revocation_confirmed = false`, пока реальный
Linear OAuth revoke не прошёл acceptance. Не считайте локальный disconnect
подтверждением удаления remote grant.

## Диагностика состояний

| Состояние | Значение | Действие |
| --- | --- | --- |
| `toolhive_missing` | Node не нашёл pinned ToolHive binary | Установить `0.40.0` или задать `UPRAVA_TOOLHIVE_BINARY`, затем перезапустить Node |
| `missing_auth` | Runtime запущен без usable Linear authorization | Завершить разрешённый OAuth flow после открытия acceptance gate |
| `starting` / `installing` | Reconciler ещё не получил terminal actual state | Проверить heartbeat и дождаться следующего dependency report |
| `degraded` | Runtime отвечает, но health/schema path ненадёжен | Проверить Node logs и безопасный ToolHive status без credential material |
| `failed` | Reconciliation или bounded MCP call завершились ошибкой | Сопоставить `error_code`, Node command и tool-call trace |
| `stopped` | Desired state отключён | Включить connection только через разрешённый Connect/Reconnect flow |
| `node_offline` | Core закрыл availability по heartbeat | Восстановить Node/control channel; не подменять actual state вручную |
| `policy_blocked` / `permission_denied` | Core policy запретила visibility или Execute | Проверить scope, actor, risk и approval policy |
| `schema_changed` | Schema hash изменился после Inspect | Повторить Inspect и только затем Execute |

## Метрики и безопасные логи

Core `/api/v1/metrics` публикует low-cardinality counters:

- `uprava_core_tool_registry_searches_total`;
- `uprava_core_tool_execution_requests_total`;
- `uprava_core_tool_execution_failures_total`;
- `uprava_core_tool_policy_denials_total`;
- `uprava_core_tool_dependency_errors_total`;
- `uprava_core_mcp_leases_issued_total`;
- `uprava_core_mcp_lease_rejections_total`.

Диагностика должна использовать identifiers, states, error codes, hashes и
bounded/redacted summaries. Нельзя писать OAuth URL state, verifier, bearer
lease, access/refresh token, credential path или raw secret-bearing upstream
payload в logs, events, tool-call summaries или UI.

## Проверки перед handoff

```sh
make l
make c
```

Для будущего release closure дополнительно обязательны clean migration from
`0.2.10`, dependency license/advisory review, threat-model review и реальный
opt-in ToolHive + Linear E2E. До успешного E2E нельзя менять implementation
version на `0.2.11` или добавлять release в shipped ledger.
