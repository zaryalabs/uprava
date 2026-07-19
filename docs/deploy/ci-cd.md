# Дизайн CI/CD

Статус: реализованный production contract.

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
privileges выполнено для root-required операций: deploy и finalize проходят
через два фиксированных root-owned helper без аргументов, которые сверяют phase
worktree с публичным `origin/main`; общего passwordless sudo у runner нет.

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

## Локальные проверки commit и push

Локальные hooks дают раннюю обратную связь, но не переносят ответственность с
CI. Hook `pre-commit` запускает проверки форматирования, lint и типов через
`make l`; Rust и Web test suites на каждом commit не запускаются. Это сохраняет
достаточно быстрый внутренний цикл разработки.

Hook `pre-push` запускает `make push-check`. Эта цель одновременно является
каноническим набором source checks для успешного prepare path ветки `main` и
включает unit и integration tests, production Web build, MSRV, dependency
checks и Web E2E tests. Локальный hook работает в host-окружении разработчика,
поэтому авторитетным воспроизводимым запуском остаётся CI внутри pinned prepare
container.

Локальный pre-push намеренно не вызывает `ci/run.sh`: lifecycle временного CI
worktree, disk preflight, очистка устаревших workspaces, release build,
публикация в registry, deployment и production validation остаются только на
сервере. Недостающие локальные зависимости следует явно устанавливать через
`make init`, а не неявно во время push. Инициализация также устанавливает MSRV
toolchain, необходимый общему набору проверок.

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
не запускает production validation и не чистит сервер. Большая clean-state deployment
rehearsal не требуется для каждой сборки; достаточно focused image startup tests.

## Deploy

`deploy` намеренно остаётся небольшим. Он:

1. устанавливает product-owned operations files;
2. устанавливает candidate manifest из `build`;
3. сохраняет активный release как проверяемый rollback target;
4. активирует candidate и скачивает digest-pinned artifacts;
5. применяет Core/Web через Compose;
6. устанавливает и перезапускает Node binary через разрешённый systemd unit.

Он не запускает smoke, не проверяет business projections, не очищает artifacts,
не сбрасывает SQLite state и сам не проверяет результат. Успешный deploy
означает только, что запрошенный release применён; automatic rollback относится
к readiness gate следующей фазы.

## Finalize

`finalize` запускается после `deploy` как отдельная фаза. Он:

1. ожидает health Core и Web;
2. проверяет public route и ожидаемый release SHA;
3. проверяет active state Node systemd unit;
4. ожидает свежий Node heartbeat и сверяет Node version;
5. при ошибке readiness возвращает совместимый предыдущий release либо
   деактивирует failed first candidate;
6. после успешной readiness-проверки удаляет только ограниченную историю Uprava
   releases и images;
7. выводит краткую production summary.

Finalize использует stable operational interfaces. Он не записывает internal
SQLite metadata, не требует конкретной business projection и не зависит от
private table layouts. Read-only SQLite integrity check может существовать как
отдельная diagnostic command, но не является обязательным release gate.

Если readiness-часть `finalize` падает, workflow становится красным, сохранённый
release той же family и с теми же state slots автоматически активируется снова,
а Core/Web/Node перезапускаются. При отсутствии безопасного target failed first
candidate останавливается и active links удаляются. Ошибка retention после
успешного readiness gate не откатывает валидный release. Alert delivery остаётся
вне scope.

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
содержит одноразовую reset instruction, а `deploy` не принимает небезопасный
reset switch.

Disposable SQLite state до перехода был один раз удалён при clean rebuild
production. В обычном CI/CD нет state-reset target, flag или manifest field.
Любой будущий maintenance reset должен быть отдельным operator workflow с typed
confirmation и проверкой точных paths и никогда не вызываться CI. Scoped
auto-enrollment для точного production Node name остаётся независимой постоянной
bootstrap policy.
