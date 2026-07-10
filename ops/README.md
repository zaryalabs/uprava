# Uprava Server Ops

These files are copied to `/opt/apps/uprava` on the Zarya server.

The first bootstrap can build a release from an existing checkout:

```bash
cd /opt/apps/uprava
make build-local RELEASE="$(date -u +%Y%m%d-%H%M%S)-$(git -C /srv/uprava-workspaces/uprava rev-parse --short HEAD)"
make activate RELEASE="<release-id>"
make deploy
```

Normal CI/CD publishes immutable Core/Web images plus the `uprava-node` GHCR
artifact, copies the release manifest into `builds/releases/`, and calls
`make activate RELEASE=<release-id>` followed by `make deploy`.

To return to an earlier release, verify a backup first, then run the explicit
rollback preflight. It refuses a missing or already-active manifest and only
switches the release symlinks; deploy and smoke remain mandatory follow-up
steps:

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
