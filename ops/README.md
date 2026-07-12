# Uprava Server Ops

These files are copied to `/opt/apps/uprava` by the automatic `main` delivery
job. Production releases are never built or activated manually on the server.

Normal CI/CD publishes immutable Core/Web images plus the `uprava-node` GHCR
artifact and a release manifest. The manifest couples those artifacts to the
stable Core state directory, Core config, Node config and Node state path.
The delivery job validates stable paths, activates the digest-pinned manifest,
applies a coordinated state epoch when required, starts the runtime and accepts
the deployment only after functional Core/Web/Node smoke. Rollback remains an
optional break-glass operation between releases that share the schema contract.

To return to an earlier release, verify a backup first, then run the explicit
rollback preflight. It refuses a missing or already-active manifest and only
switches the artifact, configuration and matching-state symlinks; deploy and
smoke remain mandatory follow-up steps:

```bash
make backup
make rollback RELEASE="<previous-release-id>"
make deploy
make smoke
```

For registry-based releases, `make pull` pulls Core/Web images, pulls the
`UPRAVA_NODE_ARTIFACT` image from `.env.release`, extracts
`/usr/local/bin/uprava-node` into `builds/releases/<release-id>/`, verifies
`UPRAVA_NODE_SHA256`, and then `make deploy` updates Compose and restarts the
product-owned systemd unit.
