# Uprava Deployment

Status: `working-position`

This document describes the Uprava deployment model on the Zarya server from
inside the product repository. It exists so work done directly in `uprava`,
without the wider Oreol workspace open, still has enough context for why
Core/Web deploy in Docker while Node Daemon runs bare-metal through systemd, and
how that connects to CI/CD.

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

### Breaking 0.2.0 State And Configuration Cut

0.2.0 deliberately starts from clean stable state paths. There is no in-place
0.1.8 migration and no compatibility rollback to 0.1.8. Before activation the
operator creates and verifies an offline legacy archive. The new binaries then
initialize `state/core/core.sqlite` and `/var/lib/uprava-node/node.sqlite`; a
0.1.8 binary must never open those files.

Every immutable release manifest declares the stable Core and Node state and
configuration paths. Activation refuses any other production paths. Rollback
is supported only between later releases that share the current schema
contract. Immutable artifacts remain addressed by release id while mutable
state does not contain a package version in its pathname.

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

### Container Runtime Identity

Release images do not run application processes as root. Core and the Node
artifact image use the dedicated `uprava` user (UID/GID `10001`); Web uses the
base image's `node` user. The images create and own their runtime directories
before switching users. Core persists SQLite and logs under `/data`, while the
Node image defaults to `/var/lib/uprava-node` for state and `/workspaces`
for workspace access. Production Compose or host mounts must preserve write
access for the corresponding non-root identity; do not solve permission
failures by overriding `USER` to root.

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

The self-hosting workspace is deliberately separate from the runtime
installation:

```text
/opt/apps/uprava/                 # deployed runtime installation
/srv/uprava-workspaces/uprava/    # editable git checkout for agent work
```

`/srv/uprava-workspaces` should be created by root as a dedicated workspace
boundary, but it should be writable by the `uprava` group and carry default ACLs
so everything under `/srv/uprava-workspaces/*` remains writable by the Node
Daemon user. Configure the Node Daemon with:

```text
UPRAVA_NODE_WORKSPACES=/srv/uprava-workspaces
```

The `uprava` user may have a GitHub deploy key or machine credential that can
push feature branches to `zaryalabs/uprava`. It must not use root's key or a
personal operator credential.

Agent sessions may edit the workspace clone, run checks, commit and push
feature branches. They must not directly edit `/opt/apps/uprava`, production
`.env` files, systemd units, proxy configuration, active release symlinks,
volumes or backups. Production changes should reach the server through branch,
review, merge and CI/CD, not through direct runtime mutation.

The detailed user-facing flow lives in
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

The host artifact transport is the `uprava-node` GHCR image. CI extracts
`/usr/local/bin/uprava-node` from that image to calculate `UPRAVA_NODE_SHA256`.
Server deploy pulls the same digest-pinned image, extracts the binary into the
active release directory and verifies the checksum before restarting systemd.
Deploy must not download a mutable `latest` daemon binary.

## CI/CD Stages

The accepted target pipeline is defined in [`ci-cd.md`](ci-cd.md) as
`prepare -> build -> deploy -> finalize`. The details below describe the current
transitional implementation until that four-phase contract is adopted. In the
target design, smoke and retention belong to `finalize`, while `deploy` only
applies the selected release.

The top-level product contract stays:

```text
prepare -> build -> push -> deploy
```

Pull requests run checks only. Every successful update to `main` builds and
publishes one immutable Git-SHA release and then activates that exact release in
production. There is no manual production deploy path or `workflow_dispatch`
activation. The delivery job owns temporary registry credentials and removes
its workspace and credentials after every outcome.

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

The GitHub Actions delivery job runs automatically after all checks succeed on
`main` and performs only the repository-owned release and server entrypoints:

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
- `state-transition` - reset the coordinated Core/Node SQLite state once when
  the immutable manifest declares a new state epoch.
- `deploy` - run `pull`, state transition, `up`, `status`, functional `smoke`
  and project-scoped retention.
- `retention` - keep bounded Uprava releases and images without pruning other
  products on the shared host.
- `rollback` - validate and activate a selected prior release only. It does not
  deploy or smoke-test automatically; those follow-up actions stay explicit.

## Deploy Order

Default release activation:

1. The successful `main` delivery installs the digest-pinned release manifest
   into `/opt/apps/uprava/builds/releases/`.
2. `make activate RELEASE=<release-id>` updates `.env.release` and `current`.
3. `make pull` fetches images and the Node artifact.
4. Core/Web update through Compose.
5. Node Daemon restarts through the approved product-owned systemd path.
6. Scoped auto-enrollment accepts only the Node display name declared by the
   release manifest.
7. Smoke checks verify Core, public Web, writable SQLite, state epoch, Node
   version, workspace projection and fresh heartbeat.
8. Retention removes old Uprava release artifacts and image references.

Core/Web and Node should share one release id because the control protocol is a
product contract. Ordinary releases should tolerate the short mixed-version
window during restart. Breaking protocol changes need explicit release notes and
maintenance planning.

For protocol v2, the release id is Git-SHA-based and Core, Web and Node move as
one coordinated release. The first activation initializes clean stable state;
it never imports the 0.1.8 database or Node JSON state.

## Smoke Checks

Minimum production smoke:

- Core `/api/v1/health` responds internally.
- Web route responds through the public origin.
- Core can access and write its persistent state.
- Node Daemon process is active.
- Node heartbeats to Core after restart.
- Node reports the version declared by the release manifest.
- Node projects at least one allowed workspace into Core.
- Core and Node state epoch markers match the release manifest.
- One central metric and one central Node log are visible after observability is
  wired.

## Runner And Privileges

The current self-hosted runner is a trusted privileged runner. Docker group
membership is an explicitly accepted controlled-development risk for `0.2.2`.
Pull requests from forks are not executed on it, and production credentials are
available only to the automatic `main` delivery job through a temporary Docker
configuration.

The intended later hardening model remains:

- runner can copy release manifests into `/opt/apps/uprava/builds/releases/`;
- runner can call `/opt/apps/uprava/Makefile`;
- any systemd action goes through a narrow sudoers rule or root-owned wrapper;
- wrapper validates release id, installation path and allowed service name;
- allowed service name is `uprava-node.service`, not arbitrary systemd units.

Adding/changing the installed systemd unit, Linux users/groups, workspace
permissions or sudoers policy is a manual server setup step, not ordinary CI/CD
deploy.

The `uprava` Unix user used by the Node Daemon is not the deploy runner. It may
write the allowed workspace checkout and push branches when Git credentials are
configured, but it should not have sudo rights for arbitrary systemd, Docker,
proxy or production file operations.

## Rollback

Rollback has an explicit preflight target and must be followed by the normal
deploy and smoke gates:

```bash
cd /opt/apps/uprava
make backup                         # capture/verify state first
make rollback RELEASE=<previous-release-id>
make deploy
make smoke
make status
```

`make rollback` requires `RELEASE`, verifies that the selected
`builds/releases/<release-id>.env.release` exists, refuses to select the
currently active release, and then performs the existing `activate` symlink
switch. It intentionally does not run Compose, systemd, pull, migration or
smoke commands. The operator must confirm a usable backup before activation and
must run `make deploy` and `make smoke` explicitly afterwards.

The 0.2.0 breaking release cannot roll back to 0.1.8. Rollback is available
only between later releases that explicitly share the stable state schema.
The verified 0.1.8 archive is evidence and break-glass material, not an active
rollback target.

### 0.2.2 Coordinated State Epoch Reset And Re-enrollment

This section documents transitional, not target, behavior. The accepted CI/CD
design removes automatic state epochs from ordinary delivery. The disposable
current SQLite state will instead be reset once as explicit server maintenance;
see [`ci-cd.md`](ci-cd.md#production-state-reset).

A release manifest declares `UPRAVA_STATE_EPOCH`. When it differs from either
installed epoch marker, deployment automatically stops Core/Node, removes only
the stable Core and Node SQLite database/WAL/SHM files, records both markers and
starts the new release. Re-running the same epoch is idempotent and preserves
state. The offline legacy archive is never modified. After reset:

The reset also removes the current Core users. After the first `0.2.2` delivery,
create the Web administrator again through the normal first-run setup. This is
the only required post-reset product action; Node enrollment and deployment
remain automatic.

1. start Core with empty `state/core` and stable Core config;
2. start Node with empty `/var/lib/uprava-node/node.sqlite` and stable config;
3. the Core auto-approves only the exact production Node display name declared
   by the immutable manifest and Node stores the new credential;
4. rebind Projects and Placements; no 0.1.8 Project, Placement, session,
   transcript or resume state is imported in place;
5. run functional smoke checks against the deployed Git SHA and Node version.

If either process sees incompatible state in its selected 0.2.0 slot, startup
must fail with an actionable incompatible-state error rather than migrate,
reinterpret or delete it automatically.

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
- stable Core state and Core config;
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
