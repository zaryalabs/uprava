# Uprava Deployment

Статус: `working-position`

Этот документ описывает deployment model Uprava на сервере Zarya изнутри
продуктового репозитория. Он нужен, чтобы при работе прямо в `uprava`, без
открытого Oreol workspace, было понятно, почему Core/Web деплоятся в Docker, а
Node Daemon запускается bare-metal через systemd, и как это связано с CI/CD.

## Source Of Truth

Platform-level rules live in Oreol:

- `docs/cicd-contract.md` - Make-first CI/CD and server operations contract.
- `docs/deployment-targets.md` - target ids, GitHub Environments and runner
  labels.
- `docs/infra-vision.md` - `/opt/infra` and `/opt/apps` platform model.

This document applies those rules to Uprava. Observability details live in
[`deployment-observability.md`](deployment-observability.md). This document owns
deployment shape, release artifacts, server Make targets and CI/CD behavior.

## Short Decision

Uprava uses a hybrid installation:

```text
Core/Web runtime units: Docker Compose
Node Daemon runtime unit: host systemd
Release manifest: .env.release
Server entrypoint: /opt/apps/uprava/Makefile
Target: zarya-main / production-main
Runner labels: self-hosted, zarya-main, geo-eu, ci
```

Core and Web belong in Docker because they are browser-facing services with
normal container deployment needs. Node Daemon belongs on the host because it
owns real workspaces, PTY/process lifecycle, local tools, provider binaries and
host credentials.

This is not an exception to the Oreol method. It is the hybrid/runtime-unit form
of the same installation contract.

## Server Installation

Target layout:

```text
/opt/apps/uprava/
  Makefile
  README.md
  compose.yaml
  .env
  .env.release -> builds/releases/<release-id>.env.release
  current -> builds/releases/<release-id>/
  builds/
    releases/
      <release-id>.env.release
      <release-id>/
        uprava-node
  systemd/
    uprava-node.service.example
  backups/
  configuration/
    core.env
  state/
    core/core.sqlite
```

Host-level files:

```text
/etc/systemd/system/uprava-node.service
/etc/uprava/node.env
/var/lib/uprava/
/var/lib/uprava-node/node.sqlite
/var/log/uprava-node/ optional local fallback logs
/srv/uprava-workspaces/ root-created workspace boundary with uprava ACL
/srv/uprava-workspaces/uprava/ editable self-hosting checkout
```

The product repository owns templates and docs for host-level files. The server
owns the installed unit file, env file, local state, workspace root and actual
workspace permissions.

### Stable State And Configuration Cut Breaking Release 0.2.0

0.2.0 намеренно начинает работу с чистыми стабильными state paths. In-place
migration с 0.1.8 и compatibility rollback на 0.1.8 отсутствуют. До activation
operator создаёт и проверяет offline legacy archive. Затем новые binaries
инициализируют `state/core/core.sqlite` и `/var/lib/uprava-node/node.sqlite`;
binary 0.1.8 никогда не должен открывать эти файлы.

Каждый immutable release manifest объявляет стабильные Core/Node state и config
paths. Activation отклоняет другие production paths. Rollback допустим только
между последующими releases с совместимым schema contract. Immutable artifacts
адресуются release id, а mutable state не содержит package version в pathname.

## Runtime Units

### Compose Units

`compose.yaml` should manage:

- `uprava-core`;
- `uprava-web`;
- Core persistent state volume or bind mount;
- `platform` Docker network attachment for Traefik.

Production should expose one public browser origin:

```text
https://uprava.zrya.io/       -> Web UI
https://uprava.zrya.io/api/v1 -> Core API
```

The Web build should use a same-origin API base such as `/api/v1` in
production. Local development may keep `http://127.0.0.1:8080/api/v1`.

### Идентичность контейнеров

Release images не запускают application processes от root. Core и image с
Node artifact используют выделенного пользователя `uprava` (UID/GID `10001`),
а Web — пользователя `node` из базового image. Images заранее создают и
назначают владельца runtime directories, затем переключаются на non-root
пользователя. Core хранит SQLite и logs в `/data`; Node image по умолчанию
использует `/var/lib/uprava-node` для state и `/workspaces` для workspace
access. Production Compose или host mounts должны сохранять write access для
соответствующего non-root identity; нельзя исправлять permission failures,
переопределяя `USER` на root.

### Systemd Unit

`uprava-node.service` should manage only the host Node Daemon. It should:

- run as the dedicated Unix user `uprava`;
- read `/etc/uprava/node.env`;
- use `/var/lib/uprava-node` for local state;
- configure explicit `UPRAVA_NODE_WORKSPACES`;
- point `UPRAVA_CORE_URL` at the production Core origin;
- restart on process failure;
- not restart just because Core is temporarily unavailable.

Installing or changing the unit file is an operational contract change, not an
ordinary release deploy. Ordinary deploy may restart the declared product-owned
unit through the installation Makefile or approved deploy wrapper.

### Self-Hosting Workspace

Self-hosting workspace намеренно отделен от runtime installation:

```text
/opt/apps/uprava/                 # deployed runtime installation
/srv/uprava-workspaces/uprava/    # editable git checkout for agent work
```

`/srv/uprava-workspaces` should be created by root как dedicated workspace
boundary, но он должен быть writable by `uprava` group и иметь default ACLs,
чтобы все под `/srv/uprava-workspaces/*` оставалось writable для Node Daemon
user. Configure the Node Daemon with:

```text
UPRAVA_NODE_WORKSPACES=/srv/uprava-workspaces
```

У пользователя `uprava` может быть GitHub deploy key or machine credential,
который умеет push-ить feature branches в `zaryalabs/uprava`. Нельзя
использовать root key или personal operator credential.

Agent sessions могут редактировать workspace clone, запускать checks, commit
and push feature branches. Они не должны напрямую редактировать
`/opt/apps/uprava`, production `.env` files, systemd units, proxy
configuration, active release symlinks, volumes or backups. Production changes
должны попадать на сервер через branch, review, merge and CI/CD, а не через
direct runtime mutation.

Detailed user-facing flow lives in
[`self-hosting-golden-path.md`](self-hosting-golden-path.md).

## Release Manifest

Uprava should use `.env.release`, not `.env.images`, because one release pins
both Docker images and the host daemon artifact.

Example:

```text
UPRAVA_RELEASE_ID=20260708-120000-abcdef0
UPRAVA_RELEASE_SHA=abcdef0123456789
UPRAVA_RELEASE_AT=2026-07-08T12:00:00Z

UPRAVA_CORE_IMAGE=ghcr.io/zaryalabs/uprava-core@sha256:<digest>
UPRAVA_WEB_IMAGE=ghcr.io/zaryalabs/uprava-web@sha256:<digest>

UPRAVA_NODE_ARTIFACT=ghcr.io/zaryalabs/uprava-node@sha256:<digest>
UPRAVA_NODE_SHA256=<sha256>
UPRAVA_NODE_VERSION=0.1.x
UPRAVA_STATE_EPOCH=0.2.2
UPRAVA_AUTO_APPROVE_NODE_NAME='Zarya Server'
```

Host artifact transport теперь зафиксирован: это GHCR image `uprava-node`. CI
извлекает `/usr/local/bin/uprava-node` из этого image, чтобы посчитать
`UPRAVA_NODE_SHA256`. Server deploy тянет тот же digest-pinned image,
извлекает binary в active release directory and verifies the checksum перед
restart systemd. Deploy must not download a mutable `latest` daemon binary.

## CI/CD Stages

The top-level product contract stays:

```text
prepare -> build -> push -> deploy
```

Pull requests запускают только checks. Каждое успешное обновление `main`
собирает и публикует один immutable Git-SHA release, после чего автоматически
активирует именно его в production. Manual production deploy и
`workflow_dispatch` activation отсутствуют. Delivery job использует temporary
registry credentials и удаляет workspace и credentials при любом исходе.

### `prepare`

Expected checks:

- Rust format, clippy and tests;
- Web format, lint and tests;
- Docker/Compose config validation for production ops assets;
- systemd unit template validation when present;
- smoke scripts syntax checks;
- secret-sensitive files stay uncommitted.

### `build`

Expected outputs:

- immutable Core image;
- immutable Web image;
- host `uprava-node` artifact with checksum;
- release metadata needed to write `.env.release`.

### `push`

Expected outputs:

- pushed Core and Web image digests;
- published digest-pinned Node artifact image;
- `builds/releases/<release-id>.env.release`.

### `deploy`

GitHub Actions delivery job автоматически запускается после успешных checks в
`main` и использует только repository-owned release/server entrypoints:

```bash
make push
make install-ops INSTALL_DIR=/opt/apps/uprava SUDO=sudo
make install-release-manifest INSTALL_DIR=/opt/apps/uprava SUDO=sudo \
  RELEASE_ID="${RELEASE_ID}"
make deploy INSTALL_DIR=/opt/apps/uprava SUDO=sudo \
  RELEASE_ID="${RELEASE_ID}" DEPLOY_MODE=local
```

Workflow YAML must not inline `docker compose`, `systemctl`, migration or smoke
logic. The installation Makefile owns those operations.

## Server Makefile Contract

From outside, the Makefile behaves like any other Oreol product installation.
Internally, targets may operate on both Compose and systemd units.

Expected targets:

```text
help
ps
status
logs
releases
activate
pull
up
down
restart
state-transition
deploy
smoke
retention
backup
rollback
restore
```

Suggested meanings:

- `pull` - pull Core/Web images and fetch/verify the Node artifact.
- `activate` - update `.env.release` and `current` to point to the selected
  release manifest and extracted host artifact directory.
- `up` - apply Compose Core/Web state and start/restart `uprava-node.service`.
- `status` - show Compose status and `systemctl status uprava-node.service`.
- `logs` - show Compose logs and `journalctl -u uprava-node`.
- `state-transition` - один раз сбрасывает coordinated Core/Node SQLite state,
  когда immutable manifest объявляет новый state epoch.
- `deploy` - запускает `pull`, state transition, `up`, `status`, functional
  `smoke` и project-scoped retention.
- `retention` - ограничивает Uprava releases/images, не очищая другие продукты
  shared host.
- `rollback` - проверяет и активирует выбранный предыдущий release только.
  Deploy и smoke остаются явными последующими действиями.

## Deploy Order

Default release activation:

1. Успешный `main` delivery устанавливает digest-pinned release manifest в
   `/opt/apps/uprava/builds/releases/`.
2. `make activate RELEASE=<release-id>` updates `.env.release` and `current`.
3. `make pull` fetches images and the Node artifact.
4. Core/Web update through Compose.
5. Node Daemon restarts through the approved product-owned systemd path.
6. Scoped auto-enrollment принимает только Node display name из release
   manifest.
7. Smoke checks проверяют Core, public Web, writable SQLite, state epoch, Node
   version, workspace projection и свежий heartbeat.
8. Retention удаляет старые Uprava release artifacts и image references.

Core/Web and Node should share one release id because the control protocol is a
product contract. Ordinary releases should tolerate the short mixed-version
window during restart. Breaking protocol changes need explicit release notes and
maintenance planning.

Для protocol v2 release id основан на Git SHA, а Core, Web and Node переходят
как один coordinated release. Artifacts `0.2.0-rc.N` никогда не активируются с
state/config slot 0.1.8.

## Smoke Checks

Minimum production smoke:

- Core `/api/v1/health` responds internally.
- Web route responds through the public origin.
- Core can access and write its persistent state.
- Node Daemon process is active.
- Node heartbeats to Core after restart.
- Node reports version из release manifest.
- Node projects как минимум один allowed workspace в Core.
- Core and Node state epoch markers совпадают с release manifest.
- One central metric and one central Node log are visible after observability is
  wired.

## Runner And Privileges

Текущий self-hosted runner считается trusted privileged runner. Membership в
Docker group — явно принятый controlled-development risk для `0.2.2`. Pull
requests from forks на нём не выполняются, а production credentials доступны
только automatic `main` delivery job через temporary Docker configuration.

Целевая later hardening model остаётся следующей:

- runner can copy release manifests into `/opt/apps/uprava/builds/releases/`;
- runner can call `/opt/apps/uprava/Makefile`;
- any systemd action goes through a narrow sudoers rule or root-owned wrapper;
- wrapper validates release id, installation path and allowed service name;
- allowed service name is `uprava-node.service`, not arbitrary systemd units.

Adding/changing the installed systemd unit, Linux users/groups, workspace
permissions or sudoers policy is a manual server setup step, not ordinary CI/CD
deploy.

Unix user `uprava`, от которого работает Node Daemon, не является deploy
runner. Он может писать в allowed workspace checkout и push-ить branches, когда
Git credentials configured, но не должен иметь sudo rights для arbitrary
systemd, Docker, proxy or production file operations.

## Rollback

Rollback использует явный preflight target и затем проходит обычные deploy и
smoke gates:

```bash
cd /opt/apps/uprava
make backup                         # сначала снять/проверить state backup
make rollback RELEASE=<previous-release-id>
make deploy
make smoke
make status
```

`make rollback` требует `RELEASE`, проверяет наличие
`builds/releases/<release-id>.env.release`, отказывается выбирать уже
активный release и только затем выполняет существующее переключение symlink
через `activate`. Target намеренно не запускает Compose, systemd, pull,
migration или smoke. Оператор должен подтвердить пригодный backup до
activation и явно выполнить `make deploy` и `make smoke` после неё.

Для breaking release 0.2.0 rollback вместе выбирает release manifest 0.1.8,
Core config, Core state, Node config и Node JSON state. Он никогда не запускает
old binary с new schema. Сохранённые slots 0.1.8 остаются неизменными до
acceptance 0.2.0. Работа, созданная только в 0.2.0, после rollback отсутствует;
эта loss boundary должна быть показана до activation.

### Coordinated State Epoch Reset 0.2.2 И Re-enrollment

Release manifest объявляет `UPRAVA_STATE_EPOCH`. Если он отличается от любого
installed epoch marker, deployment автоматически останавливает Core/Node,
удаляет только stable Core and Node SQLite database/WAL/SHM files, записывает
оба markers и запускает новый release. Повторный deploy того же epoch
идемпотентен и сохраняет state. Offline legacy archive не изменяется. После
reset:

Reset также удаляет текущих Core users. После первого delivery `0.2.2` нужно
снова создать Web administrator через обычный first-run setup. Это единственное
обязательное product action после reset; Node enrollment и deployment остаются
автоматическими.

1. запустить Core с empty Core slot 0.2.0 и Core config 0.2.0;
2. запустить Node с empty SQLite slot 0.2.0 и Node config 0.2.0;
3. Core auto-approve только точный production Node display name из immutable
   manifest, после чего Node сохраняет новый credential;
4. заново bind Projects and Placements; Project, Placement, session,
   transcript или resume state 0.1.8 не импортируются in place;
5. запустить functional smoke checks против deployed Git SHA and Node version.

Если процесс видит incompatible state в выбранном slot 0.2.0, startup обязана
завершиться actionable incompatible-state error, а не автоматически migrate,
reinterpret или удалить state.

## What Belongs Where

Product repository:

- Dockerfiles and build logic;
- `ops/compose.yaml`;
- `ops/Makefile`;
- systemd unit template/reference;
- `.env.example`;
- deploy, backup and restore docs;
- CI workflow.

Server-owned state:

- `/opt/apps/uprava/.env`;
- active `.env.release` symlink;
- active `current` release symlink;
- stable Core state и Core config;
- installed systemd unit;
- `/etc/uprava/node.env`;
- `/var/lib/uprava/`;
- `/var/lib/uprava-node/node.sqlite`;
- `/srv/uprava-workspaces/`;
- `/srv/uprava-workspaces/uprava/`;
- real workspace files and credentials;
- backups.

Oreol:

- platform CI/CD contract;
- deployment target catalog;
- shared observability and proxy stacks;
- umbrella docs and local submodule workspace.

## Open Questions

- Whether production Web remains a separate container or Core serves built
  static assets later.
- Exact Git credential mechanism for the `uprava` server user.
- Whether release id should be reported by Node heartbeat before the first
  production deploy.
