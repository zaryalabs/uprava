# Развёртывание Uprava

Статус: реализовано.

Uprava использует гибридную production-инсталляцию: Core и Web работают в
Docker Compose, а Node Daemon запускается systemd от отдельного Unix-пользователя
`uprava`. Production изменяется только успешным push в `main` через
четырёхфазный контракт из [`ci-cd.md`](ci-cd.md).

## Устойчивые host prerequisites

Host administrator владеет inputs вне любых release directories:

```text
/etc/uprava/core.env        root:root 0600
/etc/uprava/node.env        root:root 0600
/srv/uprava-workspaces/     shared Node workspace boundary
Docker, Compose, systemd, TLS и shared platform network
Unix-пользователь и группа uprava
GitHub Actions runner с labels self-hosted, zarya-main, geo-eu, ci
/usr/local/sbin/uprava-ci-root-{deploy,finalize} — root-owned phase gates
/etc/sudoers.d/uprava-ci-root только с этими двумя командами без аргументов
```

`core.env` содержит устойчивые route и Core runtime settings. `node.env`
содержит Core URL, точное production display name, workspace allow-list,
logging, provider и stable Node state settings. Значения не копируются в
repository или CI logs.

Root helpers устанавливаются out of band из `ops/uprava-ci-root`; runner не
может их заменить. Перед запуском repository-controlled deploy или finalize
каждый helper требует ровно один worktree нужной фазы и проверяет совпадение SHA
из manifest, HEAD worktree и публичного `origin/main`. Sudoers разрешает
`runner` только две точные helper-команды без аргументов и не даёт общего sudo.

## Product-owned layout

`deploy` создаёт инсталляцию на чистом host:

```text
/opt/apps/uprava/
  Makefile
  README.md
  compose.yaml
  .env.release -> builds/releases/<release-id>.env.release
  current -> builds/releases/<release-id>/
  .env.previous -> builds/releases/<previous-release-id>.env.release
  previous -> builds/releases/<previous-release-id>/
  builds/releases/<release-id>/uprava-node
  state/core/core.sqlite
  scripts/prune-uprava-images.sh
  scripts/prune-uprava-releases.sh
  systemd/uprava-node.service

/etc/systemd/system/uprava-node.service
/var/lib/uprava-node/node.sqlite
```

Release directories содержат immutable manifests и binaries. SQLite state Core
и Node использует stable paths и сохраняется при обычных releases. Editable
self-hosting checkout остаётся в `/srv/uprava-workspaces/uprava` и не является
частью runtime installation.

## Immutable release manifest

Build публикует один shell-safe manifest с полным Git SHA, build timestamp,
stable path contract, точным Node bootstrap name, Node version, digest-pinned
Core/Web/Node image refs и checksum извлечённого Node binary. Manifest не
содержит secrets или одноразовых state instructions. GitHub Actions передаёт
его из `build` в `deploy` как artifact.

## Ответственность фаз

- `prepare` выполняет source, unit, integration, protocol, docs и focused ops
  tests внутри `ci/Dockerfile` без Docker socket. На main дополнительно идут
  MSRV, dependency/security и browser checks.
- `build` использует только host Docker engine для build, startup check и
  publication immutable artifacts, затем создаёт release manifest.
- `deploy` создаёт directories и ops files, устанавливает systemd unit и
  manifest, сохраняет согласованные manifest/binary links активного release как
  rollback target, pull-ит artifacts, проверяет Node checksum, запускает
  Core/Web и перезапускает Node. Он не проверяет health, не чистит artifacts и
  не сбрасывает state.
- `finalize` ждёт local и public health Core/Web, проверяет public Git SHA,
  systemd unit и вызывает read-only operational interface Core
  `deployment-status` для проверки Node version и heartbeat. Ошибка на этом
  readiness gate автоматически возвращает сохранённые links и перезапускает
  предыдущие Core/Web/Node. Rollback разрешён только для той же release family
  и тех же state slots. Если это первая установка или target небезопасен,
  candidate останавливается и его active links удаляются. Только после
  успешного readiness gate выполняется bounded Uprava release/image retention
  и печатается summary.

Failure readiness-части `finalize` делает workflow красным и не оставляет
невалидный release active. Ошибка retention или summary после успешной
readiness-проверки также делает workflow красным, но не откатывает уже
проверенный release. Rollback target сохраняется release retention независимо
от общего числового лимита.

## Runtime boundaries

Compose подключает Core и Web к существующей external network `platform`.
Containers работают non-root с read-only root filesystem и ограниченными
writable mounts. Node systemd unit использует `NoNewPrivileges`, strict
protected system view и explicit write access только к Node state, home и
workspace paths.

Точное production Node display name может auto-enroll при clean bootstrap. Эта
scoped policy не разрешает broad enrollment approval.

## Retention и diagnostics

Retention удаляет только старые Uprava release manifests/directories и images
из трёх Uprava repositories. Global Docker prune не выполняется. Phase
workspaces runner уникальны, удаляются unconditional cleanup и собираются GC,
если остались после прерванных jobs. Operational logs доступны через installed
target `make logs`, но installation и activation остаются CI-only.
