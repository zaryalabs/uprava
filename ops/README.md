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
