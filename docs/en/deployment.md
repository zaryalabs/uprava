# Uprava Deployment

Status: implemented.

Uprava uses a hybrid production installation: Core and Web run in Docker
Compose, while Node Daemon runs as the dedicated `uprava` Unix user through
systemd. Production is changed only by a successful push to `main` through the
four-phase contract in [`ci-cd.md`](ci-cd.md).

## Stable Host Prerequisites

The host administrator owns these inputs outside every release directory:

```text
/etc/uprava/core.env        root:root 0600
/etc/uprava/node.env        root:root 0600
/srv/uprava-workspaces/     shared Node workspace boundary
Docker, Compose, systemd, TLS and the shared platform network
uprava Unix user and group
GitHub Actions runner with self-hosted, zarya-main, geo-eu, ci labels
```

`core.env` contains stable route and Core runtime settings. `node.env` contains
the Core URL, exact production display name, workspace allow-list, logging,
provider and stable Node state settings. Values are never copied into the
repository or CI logs.

## Product-Owned Layout

`deploy` creates the product installation from an otherwise clean host:

```text
/opt/apps/uprava/
  Makefile
  README.md
  compose.yaml
  .env.release -> builds/releases/<release-id>.env.release
  current -> builds/releases/<release-id>/
  builds/releases/<release-id>/uprava-node
  state/core/core.sqlite
  scripts/prune-uprava-images.sh
  scripts/prune-uprava-releases.sh
  systemd/uprava-node.service

/etc/systemd/system/uprava-node.service
/var/lib/uprava-node/node.sqlite
```

Release directories contain immutable manifests and binaries. Core and Node
SQLite state use stable paths and survive ordinary releases. The editable
self-hosting checkout remains under `/srv/uprava-workspaces/uprava` and is never
part of the runtime installation.

## Immutable Release Manifest

Build publishes one shell-safe manifest containing the full Git SHA, build
timestamp, stable path contract, exact Node bootstrap name, Node version,
digest-pinned Core/Web/Node image references and the extracted Node checksum.
It contains no secrets and no one-shot state instruction. GitHub Actions passes
this file from `build` to `deploy` as an artifact.

## Phase Responsibilities

- `prepare` runs source, unit, integration, protocol, docs and focused ops tests
  inside `ci/Dockerfile` without a Docker socket. Main additionally runs MSRV,
  dependency/security and browser checks.
- `build` uses only the host Docker engine to build, start-check and publish
  immutable artifacts, then emits the release manifest.
- `deploy` bootstraps directories and ops files, installs the systemd unit and
  manifest, pulls artifacts, verifies the Node checksum, starts Core/Web and
  restarts Node. It does not validate health, prune, reset state or roll back.
- `finalize` waits for local and public Core/Web health, verifies the public Git
  SHA, checks the systemd unit and invokes Core's read-only
  `deployment-status` operational interface for Node version and heartbeat.
  Only then does it apply bounded Uprava release/image retention and print the
  production summary.

A failed `finalize` makes the workflow red but leaves the applied release
active. Operators fix the cause locally and deliver another `main` update;
manual completion of a partial release is not part of the contract.

## Runtime Boundaries

Compose attaches Core and Web to the existing external `platform` network.
Containers run non-root with read-only root filesystems and bounded writable
mounts. The Node systemd unit uses `NoNewPrivileges`, a strict protected system
view and explicit write access only for Node state, home and workspace paths.

The exact production Node display name may be auto-enrolled during a clean
bootstrap. This scoped policy does not enable broad enrollment approval.

## Retention and Diagnostics

Retention deletes only old Uprava release manifests/directories and images from
the three Uprava repositories. It never performs global Docker pruning. Runner
phase workspaces are unique, removed unconditionally and garbage-collected when
stale after interrupted jobs. Operational logs are available through the
installed `make logs` target, but installation and activation remain CI-only.
