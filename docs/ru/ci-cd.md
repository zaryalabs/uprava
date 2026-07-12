# Дизайн CI/CD

Статус: принятый целевой дизайн; реализация ещё не завершена.

CI/CD Uprava состоит из четырёх продуктовых фаз:

```text
prepare -> build -> deploy -> finalize
```

GitHub Actions отвечает за triggers, permissions, зависимости фаз и передачу
artifacts. В нём не должно быть длинных shell-программ. Make предоставляет
короткие публичные entry points, focused shell scripts реализуют orchestration,
а Dockerfiles фиксируют воспроизводимые execution environments.

## Границы выполнения

| Фаза | Где выполняется | Ответственность |
| --- | --- | --- |
| `prepare` | CI container | Проверки исходников и тесты без production access |
| `build` | Host Docker engine | Сборка, проверка и публикация immutable release artifacts |
| `deploy` | Production host | Установка и активация одного опубликованного release |
| `finalize` | Production host | Проверка активного release и ограниченная housekeeping |

Prepare container не получает Docker socket. Production images собираются через
host Docker engine trusted runner. Операции, которым нужны `/opt/apps/uprava`,
Compose, systemd или production state, явно считаются host-only. Docker group
access trusted self-hosted runner — принятый текущий риск; дальнейшее сужение
privileges является отдельной работой.

## Prepare

`prepare` не изменяет production. Он запускается в pinned CI image с
поддерживаемыми Rust toolchains, Node, browser dependencies и project quality
tools.

Pull requests запускают только быстрые deterministic checks:

- formatting, linting и type checking;
- Rust и Web unit/integration tests;
- protocol drift и documentation checks;
- focused tests release и operations scripts.

Успешный path `main` дополнительно запускает:

- Rust MSRV check;
- dependency и security audits;
- browser/end-to-end tests.

Отдельный полный stable Rust job не нужен, когда обычный Rust test suite уже
работает на stable. Production health, systemd, public-domain и SQLite checks не
относятся к `prepare`.

## Build

`build` запускается только после успешного обновления `main` и не изменяет
production. Он:

1. собирает production images Core, Web и Node;
2. выполняет короткие image startup/runtime checks;
3. извлекает и проверяет Node binary;
4. публикует immutable images;
5. создаёт digest-pinned release manifest;
6. публикует manifest как handoff artifact для `deploy`.

Фаза не устанавливает файлы в `/opt`, не вызывает systemd, не сбрасывает state,
не запускает production smoke и не чистит сервер. Большая clean-state deployment
rehearsal не требуется для каждой сборки; достаточно focused image startup tests.

## Deploy

`deploy` намеренно остаётся небольшим. Он:

1. устанавливает product-owned operations files;
2. устанавливает и активирует manifest из `build`;
3. скачивает digest-pinned artifacts;
4. применяет Core/Web через Compose;
5. устанавливает и перезапускает Node binary через разрешённый systemd unit.

Он не запускает smoke, не проверяет business projections, не очищает artifacts,
не сбрасывает SQLite state и не делает automatic rollback. Успешный deploy
означает только, что запрошенный release применён.

## Finalize

`finalize` запускается после `deploy` как отдельная фаза. Он:

1. ожидает health Core и Web;
2. проверяет public route и ожидаемый release SHA;
3. проверяет active state Node systemd unit;
4. ожидает свежий Node heartbeat и сверяет Node version;
5. удаляет только ограниченную историю Uprava releases и images;
6. выводит краткую production summary.

Finalize использует stable operational interfaces. Он не записывает internal
SQLite metadata, не требует конкретной business projection и не зависит от
private table layouts. Read-only SQLite integrity check может существовать как
отдельная diagnostic command, но не является обязательным release gate.

Если `finalize` падает, workflow становится красным, а release остаётся active.
Успешные build и deploy остаются видны отдельно, поэтому failure boundary
понятна. Automatic rollback и alert delivery сейчас вне scope.

## Форма workflow и репозитория

Pull requests запускают только `prepare`. Push в `main` запускает все четыре
фазы последовательно. Immutable release manifest передаётся из `build` в
`deploy` как workflow artifact; build не устанавливает его заранее на
production.

Каждая фаза обычно представлена одним именованным Make entry point:

```text
make ci-prepare
make ci-build
make ci-deploy
make ci-finalize
```

Make targets остаются тонкими. Большой control flow живёт в focused scripts с
именованными stages, grouped logs и ошибками, содержащими phase, substage,
command и exit code. Workspace preparation, disk preflight, stale-workspace
collection и cleanup являются общей script logic, а не повторяются в workflow
YAML.

Целевое владение:

```text
.github/workflows/ci.yml  events, permissions, dependencies, artifact handoff
ci/Dockerfile             pinned prepare environment
ci/run.sh                 workspace lifecycle and phase dispatch
ci/prepare.sh             source checks
ci/build.sh               release build and publication
ci/deploy.sh              host-only release activation
ci/finalize.sh            post-deploy validation and retention
Makefile                  thin public phase entry points
ops/                      installed production operations contract
```

## Сброс production state

Обычный delivery никогда не удаляет Core или Node state. Release manifest не
содержит одноразовую инструкцию `UPRAVA_STATE_EPOCH`, а `deploy` не принимает
небезопасный переключатель `RESET_STATE=1`.

Текущий disposable SQLite state можно один раз сбросить как явную server
maintenance operation перед включением новой delivery model. Operator
останавливает Core и Node и удаляет только документированные SQLite, WAL и SHM
files Core/Node. Это не manual deployment: последующая установка release всё
равно автоматически запускается из `main`.

Reusable maintenance helper можно добавить позже только с typed confirmation,
проверкой точных путей и отказом работать при активных services. CI никогда его
не вызывает. Scoped auto-enrollment для точного production Node name может
остаться постоянной policy и не зависит от state reset.

Существующие automatic state-reset code и объединённый deploy/smoke/retention
target являются transitional implementation и должны быть удалены при
реализации этого дизайна.
