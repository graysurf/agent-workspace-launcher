# agent-workspace-launcher (Docker Hub runtime image)

`graysurf/agent-workspace-launcher` publishes the same Rust CLI used by Homebrew/source installs.

Runtime behavior is dual-backend:

- default: `container`
- fallback: `host` (via `--runtime host` or env override)

## What this image does

- Runs the standard command surface: `create`, `ls`, `exec`, `rm`, `reset`, `auth`, `tunnel`.
- Uses container runtime by default, so it needs host Docker access when executing lifecycle commands.
- Creates workspace containers from `graysurf/agent-env` (override with `--image`, `AGENT_ENV_IMAGE`, or `CODEX_ENV_IMAGE`).

## Runtime selectors and env contract

- Runtime flag: `--runtime container|host`
- Runtime env: `AGENT_WORKSPACE_RUNTIME` (primary), `AWL_RUNTIME` (compat)
- Container image env: `AGENT_ENV_IMAGE` (primary), `CODEX_ENV_IMAGE` (compat)

Selection precedence: `--runtime` > `AGENT_WORKSPACE_RUNTIME` > `AWL_RUNTIME` > default `container`.

## Quick start

```sh
docker pull graysurf/agent-workspace-launcher:latest
docker pull graysurf/agent-env:latest
```

Run with default runtime (`container`) using host Docker daemon:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e AGENT_ENV_IMAGE=graysurf/agent-env:latest \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/REPO
```

Force host fallback runtime (for troubleshooting contract behavior):

```sh
docker run --rm -it \
  -e AGENT_WORKSPACE_RUNTIME=host \
  graysurf/agent-workspace-launcher:latest \
  ls
```

## Wrapper compatibility

Wrapper scripts remain available as compatibility helpers:

- `scripts/awl_docker.zsh`
- `scripts/awl_docker.bash`

These wrappers expose `awl_docker` and pass through `AWL_DOCKER_*` options.

## Tags

- `latest`
- `sha-<short_sha>`
- `vX.Y.Z` (release tags when enabled by release policy)

## Security note

Mounting `/var/run/docker.sock` gives root-equivalent control of the host Docker daemon. Use only on trusted hosts and with trusted images/configuration.

## Canonical docs

- Repository: <https://github.com/graysurf/agent-workspace-launcher>
- Install guide: `docs/guides/01-install.md`
- Full user guide: `docs/guides/README.md`
