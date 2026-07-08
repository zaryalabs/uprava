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
Runner labels: self-hosted, zarya-main, geo-eu, deploy
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
  volumes/
    core-data/
```

Host-level files:

```text
/etc/systemd/system/uprava-node.service
/etc/uprava/node.env
/var/lib/uprava/
/var/lib/uprava-node/node.json
/var/log/uprava-node/ optional local fallback logs
/srv/uprava-workspaces/ root-created workspace boundary with uprava ACL
/srv/uprava-workspaces/uprava/ editable self-hosting checkout
```

The product repository owns templates and docs for host-level files. The server
owns the installed unit file, env file, local state, workspace root and actual
workspace permissions.

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

UPRAVA_NODE_ARTIFACT=ghcr.io/zaryalabs/uprava-node:sha-abcdef0
UPRAVA_NODE_SHA256=<sha256>
UPRAVA_NODE_VERSION=0.1.x
```

The exact host artifact transport can be GHCR OCI artifact, GitHub Release
asset, or another explicitly documented immutable store. The release manifest
must contain a stable artifact reference and checksum. Deploy must not download
a mutable `latest` daemon binary.

## CI/CD Stages

The top-level product contract stays:

```text
prepare -> build -> push -> deploy
```

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
- published Node artifact;
- `builds/releases/<release-id>.env.release`.

### `deploy`

The GitHub Actions deploy job should be manual `workflow_dispatch` and should
only:

```bash
install -m 644 "builds/releases/${RELEASE_ID}.env.release" \
  "/opt/apps/uprava/builds/releases/${RELEASE_ID}.env.release"
cd /opt/apps/uprava
make activate RELEASE="${RELEASE_ID}"
make deploy
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
deploy
smoke
backup
restore
```

Suggested meanings:

- `pull` - pull Core/Web images and fetch/verify the Node artifact.
- `activate` - update `.env.release` and `current` to point to the selected
  release manifest and extracted host artifact directory.
- `up` - apply Compose Core/Web state and start/restart `uprava-node.service`.
- `status` - show Compose status and `systemctl status uprava-node.service`.
- `logs` - show Compose logs and `journalctl -u uprava-node`.
- `deploy` - run `pull`, optional migrations, `up`, `status` and `smoke`.
- `rollback` if added later - activate previous release and run the same
  deploy path.

## Deploy Order

Default release activation:

1. Copy the release manifest to `/opt/apps/uprava/builds/releases/`.
2. `make activate RELEASE=<release-id>` updates `.env.release` and `current`.
3. `make pull` fetches images and the Node artifact.
4. Core/Web update through Compose.
5. Node Daemon restarts through the approved product-owned systemd path.
6. Smoke checks verify Core, Web and Node heartbeat/readiness.

Core/Web and Node should share one release id because the control protocol is a
product contract. Ordinary releases should tolerate the short mixed-version
window during restart. Breaking protocol changes need explicit release notes and
maintenance planning.

## Smoke Checks

Minimum production smoke:

- Core `/api/v1/health` responds internally.
- Web route responds through the public origin.
- Core can access its persistent state.
- Node Daemon process is active.
- Node heartbeats to Core after restart.
- Node reports expected version or release id when that field is implemented.
- One central metric and one central Node log are visible after observability is
  wired.

## Runner And Privileges

The deploy runner must not receive broad host control.

Allowed model:

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

Rollback should use the same path as deploy:

```bash
cd /opt/apps/uprava
make activate RELEASE=<previous-release-id>
make deploy
```

Rollback works only if the previous release manifest still points to available
Core/Web images and Node artifact, and if Core state is compatible. Any release
that changes durable state must document rollback limits.

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
- persisted Core data;
- installed systemd unit;
- `/etc/uprava/node.env`;
- `/var/lib/uprava/`;
- `/var/lib/uprava-node/node.json`;
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

- Exact artifact transport for `uprava-node`: GHCR OCI artifact, GitHub Release
  asset, or another registry.
- Whether production Web remains a separate container or Core serves built
  static assets later.
- Exact Git credential mechanism for the `uprava` server user.
- Whether release id should be reported by Node heartbeat before the first
  production deploy.
