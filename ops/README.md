# Uprava Server Ops

These files are copied to `/opt/apps/uprava` on the Zarya server.

The first bootstrap can build a release from an existing checkout:

```bash
cd /opt/apps/uprava
make build-local RELEASE="$(date -u +%Y%m%d-%H%M%S)-$(git -C /srv/uprava-workspaces/uprava rev-parse --short HEAD)"
make activate RELEASE="<release-id>"
make deploy
```

Normal CI/CD should later publish immutable Core/Web images plus the host
`uprava-node` artifact, copy the release manifest into `builds/releases/`, and
call `make activate RELEASE=<release-id>` followed by `make deploy`.
