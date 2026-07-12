# Self-Hosting Golden Path

Status: `working-position`

This document defines the first closed improvement loop for Uprava: the
deployed Uprava instance on the Zarya server can work on a separate clone of
the `uprava` repository, prepare a branch, and push it for normal review and
CI/CD.

The goal is not automatic self-deploy. The goal is a controlled path where
Uprava can improve Uprava through the same GitHub and deployment contract as a
human developer.

## Short Decision

Use two different server locations:

```text
/opt/apps/uprava/                 # deployed runtime installation
/srv/uprava-workspaces/uprava/    # editable git checkout for agent work
```

`/opt/apps/uprava` is production runtime state. Agent sessions must not edit it
directly.

`/srv/uprava-workspaces/uprava` is a normal working clone. The host
`uprava-node` daemon may read and write it through the dedicated Unix user
`uprava`.

## Server Ownership Model

Use a dedicated server user:

```text
user:        uprava
home:        /var/lib/uprava
node state:  /var/lib/uprava-node/node.sqlite
workspace:   /srv/uprava-workspaces/*
```

The top-level workspace directory is a dedicated Uprava workspace boundary. It
is created by root, but it is group-writable by `uprava` and has inherited ACLs
so everything created under `/srv/uprava-workspaces/*` remains writable by the
daemon:

```bash
sudo install -d -o root -g uprava -m 2775 /srv/uprava-workspaces
sudo setfacl -m u:uprava:rwx,g:uprava:rwx,m:rwx /srv/uprava-workspaces
sudo setfacl -m d:u:uprava:rwx,d:g:uprava:rwx,d:m:rwx /srv/uprava-workspaces
```

Clone workspaces as `uprava` when possible:

```bash
sudo -Hu uprava git clone git@github.com:zaryalabs/uprava.git /srv/uprava-workspaces/uprava
```

If an existing workspace is imported or copied by root, apply the ACL once to
that tree instead of fixing ownership for every future operation:

```bash
sudo setfacl -R -m u:uprava:rwx,g:uprava:rwx,m:rwx /srv/uprava-workspaces/uprava
sudo find /srv/uprava-workspaces/uprava -type d -exec setfacl -m d:u:uprava:rwx,d:g:uprava:rwx,d:m:rwx {} +
```

## Node Configuration

The production Node Daemon runs as `uprava`:

```ini
[Service]
User=uprava
Group=uprava
EnvironmentFile=/etc/uprava/node.env
WorkingDirectory=/var/lib/uprava
ExecStart=/opt/apps/uprava/current/uprava-node
Restart=on-failure
RestartSec=5s
```

The first self-hosting env file should include:

```text
UPRAVA_CORE_URL=https://uprava.zrya.io
UPRAVA_NODE_STATE_PATH=/var/lib/uprava-node/node.sqlite
UPRAVA_NODE_WORKSPACES=/srv/uprava-workspaces
```

0.2.0 uses the stable `/etc/uprava/node.env` configuration and
`/var/lib/uprava-node/node.sqlite` state paths. The pre-0.2.0 config and JSON
state are retained only in the verified legacy archive, not in the active
runtime tree.

`UPRAVA_NODE_WORKSPACES` intentionally points at the workspace root so every
checkout under `/srv/uprava-workspaces/*` can be used without another server
permissions step.

For the current `codex exec` adapter, the Node launches Codex with
`--skip-git-repo-check` and
`--dangerously-bypass-approvals-and-sandbox`. This is an intentional temporary
self-hosting posture: Codex's inner Linux sandbox can be unavailable on the
server, so the effective boundary is the `uprava` Unix user, the
`UPRAVA_NODE_WORKSPACES` allow-list, inherited workspace ACLs, and the
production boundary below. A future live provider/runtime path should replace
this with finer-grained permissions and approval handling.

This is accepted audit risk P0-3, not production-grade hostile-workload
isolation. The exact follow-up and its exit criteria are tracked in
[`feature-queue.md`](feature-queue.md#6a-provider-native-persistent-execution-policy):
safe-by-default sandboxing, an explicit unsafe-mode switch, real approval
handling and visible effective policy. The 0.2.0 quality-foundation work does
not change the current launch flags.

## Git Credentials

The `uprava` user may have a GitHub deploy key or machine credential that can
push branches to `zaryalabs/uprava`. It must not use root's SSH key or a
personal operator credential.

For the first useful path, these capabilities are enough:

- `git fetch`;
- create a feature branch;
- commit local changes;
- push the feature branch.

Opening a pull request can remain a human step until GitHub integration is
added. Once GitHub tooling exists, Uprava may also open the PR through an
audited GitHub tool.

## Allowed Work Loop

The golden path is:

```text
Uprava edits /srv/uprava-workspaces/uprava
-> runs targeted checks and, when practical, make c
-> creates a branch and commit
-> pushes the branch to GitHub
-> human reviews and merges to main
-> CI/CD checks main and builds/publishes immutable artifacts
-> successful main delivery automatically activates that exact release
-> finalize verifies Core, Web, release SHA, Node version and heartbeat
-> https://uprava.zrya.io runs the updated Uprava
```

This is the intended self-improvement loop. Production changes still pass
through GitHub, review, merge and the normal CI/CD deployment contract.
Only a successful checked update to `main` activates production; PRs and failed
checks never publish or deploy.

## Production Boundary

Agent sessions may:

- edit files under `/srv/uprava-workspaces/*`;
- run project-local commands and checks there;
- create commits and push feature branches when credentials are configured;
- prepare evidence for review.

Agent sessions must not directly:

- edit `/opt/apps/uprava`;
- edit `/opt/apps/uprava/.env` or active release symlinks;
- edit `/etc/uprava/node.env`;
- change installed systemd units or sudoers policy;
- run `systemctl restart uprava-node` as an operational shortcut;
- run `docker compose up`, `restart` or `down` in `/opt/apps/uprava`;
- change Traefik/proxy configuration;
- change production volumes or backups.

This boundary does not forbid deployment after merge. It forbids bypassing the
GitHub and CI/CD path with direct server mutations.

## First Verification

Minimum acceptance test for the first server rollout:

1. Open `https://uprava.zrya.io`.
2. Confirm the host Node is visible and reachable.
3. Register or select placement `/srv/uprava-workspaces/uprava`.
4. Start a session against that placement.
5. Make a small documentation-only change.
6. Run a targeted check and capture the output.
7. Commit on a feature branch.
8. Push the branch to GitHub.
9. Confirm no files under `/opt/apps/uprava` changed outside CI/CD.

After that path works, PR creation, richer git safety checks and deployment
status blocks can be added as product features rather than manual server
shortcuts.
