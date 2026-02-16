# cws / agent-workspace guide

`cws` is a small host-side wrapper that runs the `graysurf/agent-workspace-launcher` image. The launcher container
talks to your host Docker daemon (**Docker-outside-of-Docker / DooD**) and creates **workspace containers** on the
host.

This guide focuses on the end-user experience: install `cws`, create a workspace, exec into it, and clean up.

## Requirements

- Docker Desktop or OrbStack (macOS)
- Ability to mount the Docker socket: `-v /var/run/docker.sock:/var/run/docker.sock`

## Images

The wrapper defaults to Docker Hub:

- `graysurf/agent-workspace-launcher:latest`

GHCR is also available:

- `ghcr.io/graysurf/agent-workspace-launcher:latest`

Override with `CWS_IMAGE=...`.

## Start here

1. Install `cws`: [docs/guides/cws/01-install.md](01-install.md)
2. Quickstart flow: [docs/guides/cws/02-quickstart.md](02-quickstart.md)

## Command guides

- Create workspaces: [docs/guides/cws/03-create.md](03-create.md)
- Exec into workspaces: [docs/guides/cws/04-exec.md](04-exec.md)
- Remove workspaces: [docs/guides/cws/05-rm.md](05-rm.md)
- Reset repos inside a workspace: [docs/guides/cws/06-reset.md](06-reset.md)
- VS Code tunnel: [docs/guides/cws/07-tunnel.md](07-tunnel.md)
- Update auth inside a workspace: [docs/guides/cws/08-auth.md](08-auth.md)

## Concepts and reference

- DooD rules + host mounts: [docs/guides/cws/09-dood-rules.md](09-dood-rules.md)
- Troubleshooting: [docs/guides/cws/10-troubleshooting.md](10-troubleshooting.md)
- Reference (commands + env): [docs/guides/cws/11-reference.md](11-reference.md)
- Without `cws` (direct `docker run`): [docs/guides/cws/12-agent-workspace.md](12-agent-workspace.md)
