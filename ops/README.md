# Uprava Server Ops

These files are installed from scratch by the automatic `main` pipeline.
Production releases are never built or activated manually on the server.

`deploy` validates stable host inputs in `/etc/uprava`, activates the
digest-pinned manifest, pulls Core/Web, verifies the extracted Node checksum,
starts Compose and restarts the product-owned systemd unit. It does not inspect
health, reset state, prune artifacts or roll back. The separate `finalize`
phase owns operational readiness checks and bounded Uprava-only retention.

The clean-bootstrap prerequisites are `/etc/uprava/core.env`,
`/etc/uprava/node.env`, the `uprava` user, Docker/Compose/systemd and the shared
`platform` network. Mutable Core and Node state remain outside release
directories and ordinary releases never delete them.

The host administrator also installs this directory's `uprava-ci-root` file as
both `/usr/local/sbin/uprava-ci-root-deploy` and
`/usr/local/sbin/uprava-ci-root-finalize`, owned by root and not writable by the
runner. Sudoers grants `runner` only those two no-argument commands. Each helper
verifies the phase worktree and manifest against public `origin/main` before it
executes repository-controlled deployment code as root.
