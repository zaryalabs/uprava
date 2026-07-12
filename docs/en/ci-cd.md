# CI/CD Design

Status: implemented production contract.

Uprava CI/CD has four product-level phases:

```text
prepare -> build -> deploy -> finalize
```

GitHub Actions owns triggers, permissions, phase dependencies and artifact
handoff. It must not contain long shell programs. Make exposes short public
entry points, focused shell scripts implement orchestration and Dockerfiles
define reproducible execution environments.

## Execution Boundaries

| Phase | Runs where | Responsibility |
| --- | --- | --- |
| `prepare` | CI container | Source validation and tests without production access |
| `build` | Host Docker engine | Build, verify and publish immutable release artifacts |
| `deploy` | Production host | Install and activate exactly one published release |
| `finalize` | Production host | Validate the active release and perform bounded housekeeping |

The prepare container must not receive the Docker socket. Production image
builds use the trusted runner's host Docker engine. Operations that require
`/opt/apps/uprava`, Compose, systemd or production state are explicitly
host-only. Docker group access for the trusted self-hosted runner is an accepted
current risk; further privilege narrowing is separate work.

## Prepare

`prepare` is read-only with respect to production. It runs in a pinned CI image
containing the supported Rust toolchains, Node, browser dependencies and project
quality tools.

Pull requests run only fast, deterministic checks:

- formatting, linting and type checking;
- Rust and Web unit/integration tests;
- protocol drift and documentation checks;
- focused tests for release and operations scripts.

The successful `main` path additionally runs:

- the Rust MSRV check;
- dependency and security audits;
- browser/end-to-end tests.

A full stable Rust check is not a separate job when the normal Rust test suite
already runs on stable. Production health, systemd, public-domain and SQLite
checks do not belong in `prepare`.

## Build

`build` runs only for a successful update of `main` and does not mutate
production. It:

1. builds the Core, Web and Node production images;
2. performs short image startup/runtime checks;
3. extracts and verifies the Node binary;
4. pushes immutable images;
5. produces a digest-pinned release manifest;
6. publishes that manifest as the handoff artifact for `deploy`.

The phase does not install files under `/opt`, call systemd, reset state, run
production validation or prune the server. A large clean-state deployment
rehearsal is not an every-build requirement; focused image startup tests provide
the artifact-level assurance.

## Deploy

`deploy` is intentionally small. It:

1. installs the product-owned operations files;
2. installs and activates the manifest produced by `build`;
3. pulls the digest-pinned artifacts;
4. applies Core/Web through Compose;
5. installs and restarts the Node binary through the approved systemd unit.

It does not run smoke checks, inspect business projections, prune artifacts,
reset SQLite state or roll back automatically. A successful deploy means only
that the requested release was applied.

## Finalize

`finalize` runs after `deploy` as a separate phase. It:

1. waits for Core and Web health;
2. verifies the public route and expected release SHA;
3. verifies that the Node systemd unit is active;
4. waits for a fresh Node heartbeat and checks the expected Node version;
5. prunes only bounded Uprava release and image history;
6. prints a concise production summary.

Finalize uses stable operational interfaces. It must not write internal SQLite
metadata, require a particular business projection or depend on private table
layouts. A read-only SQLite integrity check may exist as a separate diagnostic
command, but is not a mandatory release gate.

If `finalize` fails, the workflow is red and the release remains active. Build
and deploy remain separately visible as successful, so the failure boundary is
clear. Automatic rollback and alert delivery are outside the current scope.

## Workflow And Repository Shape

Pull requests run only `prepare`. A push to `main` runs all four phases in order.
The immutable release manifest passes from `build` to `deploy` as a workflow
artifact; build must not pre-install it on production.

Each phase normally has one named Make entry point:

```text
make ci-prepare
make ci-build
make ci-deploy
make ci-finalize
```

Make targets stay thin. Larger control flow lives in focused scripts with named
stages, grouped logs and errors that report the phase, substage, command and exit
code. Workspace preparation, disk preflight, stale-workspace collection and
cleanup are shared script behavior rather than duplicated workflow YAML.

The target ownership is:

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

## Production State Reset

Ordinary delivery never deletes Core or Node state. A release manifest carries
no one-off reset instruction, and `deploy` accepts no casual reset switch.

The pre-adoption disposable SQLite state was removed once during the production
clean rebuild. Ordinary CI/CD contains no state-reset target, flag or manifest
field. Any future maintenance reset must be a separate operator workflow with
typed confirmation and exact path validation and must never be called by CI.
Scoped auto-enrollment for the exact production Node name remains an independent
durable bootstrap policy.
