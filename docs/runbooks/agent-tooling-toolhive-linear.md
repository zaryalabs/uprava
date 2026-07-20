# Ручная приёмка Agent Tooling, ToolHive и Linear

Статус: `ready-for-manual-acceptance`

Runbook проверяет реальный путь Core → host Node → отдельный ToolHive service →
Linear MCP. Автоматический `dev-smoke` намеренно ограничен health/version
проверкой и не заменяет OAuth, discovery и tool call руками.

## Топология

```text
browser ──> Web :5173 ──> Core :8080
                              │
                              │ authenticated control channel
                              ▼
                      Node + Codex on host
                              │
                              │ private HTTP, 127.0.0.1:18081
                              ▼
                 ToolHive bridge + thv 0.40.0 in Compose
                              │
                              ├── OAuth callback :18765
                              └── internal MCP proxy :18766 ──> Linear
```

Node и Codex не входят в images. `thv` не устанавливается на host и не входит в
Node image. ToolHive хранит XDG/OAuth state в volume
`uprava-dev-toolhive-data`. MCP proxy `18766` не публикуется на host; Node
вызывает только bounded bridge contract. Docker socket смонтирован только в
ToolHive service, потому что ToolHive `0.40.0` создаёт container runtime manager
даже для remote MCP workload. Доступ к socket эквивалентен высокому доверию к
этому service.

## 1. Предварительные условия

- Docker Desktop/Engine запущен, `/var/run/docker.sock` доступен Compose.
- На host установлены Rust toolchain и авторизованный Codex CLI.
- Есть Linear account и workspace, где разрешён read-only acceptance call.
- Порты `8080`, `5173`, `18081` и `18765` свободны.

Полный reset удаляет также ToolHive OAuth state:

```sh
make dev-reset
```

Не выполняйте reset, если хотите проверить переживание обычного restart.

## 2. Поднять инфраструктуру

```sh
SMOKE_WEB_PASSWORD='choose-a-local-password' make dev-smoke
docker compose -f compose.dev.yaml ps
docker compose -f compose.dev.yaml exec toolhive thv version
```

Ожидания:

- Core и ToolHive имеют status `healthy`, Web доступен;
- `http://127.0.0.1:18081/api/v1/version` возвращает ToolHive `0.40.0`;
- smoke проходит без Node, Codex и Linear OAuth.

На чистом Core smoke одновременно выполняет auth setup с переданным
`SMOKE_WEB_PASSWORD`; используйте этот пароль для входа в Web. Если хотите
выполнить setup только через UI, вместо smoke запустите `make dev-up` в
отдельном терминале и проверьте version endpoint вручную.

Если ToolHive не стартует, сначала проверьте socket и логи:

```sh
docker compose -f compose.dev.yaml logs toolhive
ls -l /var/run/docker.sock
```

## 3. Запустить Node на host

В отдельном терминале:

```sh
export UPRAVA_CORE_URL=http://127.0.0.1:8080
export UPRAVA_TOOLHIVE_URL=http://127.0.0.1:18081
export UPRAVA_TOOLHIVE_TIMEOUT_SECONDS=300
export UPRAVA_NODE_WORKSPACES=/absolute/path/to/workspace
make node-r
```

Через Web завершите первоначальный auth setup/login, approve Node enrollment,
создайте placement на разрешённый workspace и session. На
`/settings/tooling` выберите эту workspace/session.

Ожидания до Linear Connect:

- observed capability `ToolHive` доступна и показывает `0.40.0`;
- `git`/`gh`/`glab` показаны как observed native capabilities, не как managed
  proxy tools;
- Linear integration имеет `missing_auth` или disconnected state;
- model-visible Uprava MCP surface содержит только `search_tools`,
  `inspect_tool`, `execute_tool`.

## 4. Пройти Linear OAuth

1. На `/settings/tooling` нажмите **Connect** у Linear.
2. Откройте полученную HTTPS-ссылку Linear в том же браузере.
3. Подтвердите нужный Linear workspace и permissions.
4. Убедитесь, что redirect пришёл на
   `http://localhost:18765/callback` и завершился без port/network error.
5. Вернитесь в Uprava и дождитесь двух-трёх heartbeat cycles.

Ожидания:

- authorization URL появляется только в текущем Web flow и не восстанавливается
  после reload;
- dependency переходит `starting` → `running`;
- `thv list --format json` внутри service содержит workload `uprava-linear`;
- на Tooling screen появились discovered Linear definitions и effective
  availability для выбранной session.

Безопасно посмотреть runtime status:

```sh
docker compose -f compose.dev.yaml exec toolhive thv list --format json
```

Не копируйте в evidence authorization URL, OAuth `state`, verifier, tokens или
содержимое ToolHive credential storage.

## 5. Проверить Search → Inspect → Execute

В выбранной Codex session попросите агента:

1. через `search_tools` найти read-only Linear tool для поиска issues;
2. через `inspect_tool` показать schema найденного tool;
3. через `execute_tool` выполнить безопасный запрос по известному тексту или
   issue identifier в разрешённом workspace;
4. не выполнять create/update/delete operations.

Ожидания:

- до Inspect полная upstream schema не появляется в model context;
- Execute использует schema hash текущей definition и проходит через Node и
  ToolHive bridge, а не прямой Linear HTTP fallback;
- `/settings/tooling` показывает terminal call, redacted summary и trace/result
  refs;
- reload сохраняет definitions, availability и terminal trace;
- Core DB/API и общие логи не содержат bearer lease, OAuth URL, credential
  paths или material; opaque non-secret reference допустим в desired state.

## 6. Проверить restart и негативные состояния

### Restart ToolHive без удаления volume

```sh
docker compose -f compose.dev.yaml restart toolhive
```

На следующей reconciliation Node должен восстановить workload из desired state.
Повторный read-only Execute должен пройти без нового Connect, если сохранённый
OAuth state usable. Если upstream требует повторную авторизацию, UI должен
показать `missing_auth`, а не делать direct fallback.

### ToolHive unavailable

```sh
docker compose -f compose.dev.yaml stop toolhive
```

После heartbeat ожидания:

- observed ToolHive и Linear availability закрываются;
- dependency получает явный `toolhive_missing`/unavailable diagnostic;
- crafted Execute отклоняется.

Вернуть service:

```sh
docker compose -f compose.dev.yaml up -d toolhive
```

### Node offline/reconnect

Остановите host `make node-r`, дождитесь offline state в Web и убедитесь, что
Execute закрыт. Запустите Node снова с тем же state file: control reconnect
должен повторно получить desired snapshot и восстановить actual state.

### Disconnect/Reconnect

Нажмите **Disconnect**. Core должен немедленно закрыть availability, Node —
вызвать ToolHive stop/remove. `remote_revocation_confirmed=false` допустим:
локальный cleanup не является доказательством отзыва remote grant. Затем
нажмите **Reconnect**, пройдите новый OAuth flow и повторите один read-only call.

## 7. Итоговый acceptance checklist

- [ ] Compose поднимает Core/Web/ToolHive, `dev-smoke` видит `0.40.0`.
- [ ] Host Node видит bridge через `UPRAVA_TOOLHIVE_URL`; host `thv` не нужен.
- [ ] Linear OAuth callback достигает container через `localhost:18765`.
- [ ] Discovery публикует bounded definitions и effective availability.
- [ ] Codex видит только три meta-tools.
- [ ] Реальный read-only Search → Inspect → Execute проходит через ToolHive.
- [ ] Trace/result refs и redacted call видны после reload.
- [ ] ToolHive restart восстанавливается или честно требует Reconnect.
- [ ] ToolHive missing и Node offline закрывают Execute без fallback.
- [ ] Disconnect удаляет workload; Reconnect создаёт новый OAuth flow.
- [ ] OAuth URL, bearer token, credential path/material не найдены в durable
  DB/log/API evidence; opaque non-secret reference не раскрывает secret.

После ручной приёмки обновите статус этого runbook и acceptance checklist в
`docs/development/agent-tooling-contracts.md`; только тогда реальный Linear E2E
можно считать закрытым.
