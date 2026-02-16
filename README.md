# agent-workspace-launcher

Launch a `Codex-ready` workspace for any repo with Docker.

- Workspace image includes `rg`, `gh`, `jq`, `git`, and other common CLI tools
- VS Code friendly: Dev Containers attach + optional VS Code tunnel
- Optional host wrapper command `aws` (zsh + bash, with completion)
- Runtime architecture: Rust `agent-workspace` CLI inside the launcher image, plus `agent-kit` low-level workspace runtime

This project packages the `agent-workspace` subcommands (`auth/create/ls/rm/exec/reset/tunnel`) as a Docker image.
No local SDK checkout is required.

This is **Docker-outside-of-Docker (DooD)**: the launcher container talks to your host Docker daemon via
`/var/run/docker.sock` and creates workspace containers on the host.

## Requirements

- Docker Desktop / OrbStack (macOS recommended)
- Docker socket mount support: `-v /var/run/docker.sock:/var/run/docker.sock`

## Quickstart

Use the `aws` wrapper (recommended):

- zsh: `source ./scripts/aws.zsh`
- bash: `source ./scripts/aws.bash`
- executable mode: put `./scripts/aws.bash` on your `PATH` (example: `cp ./scripts/aws.bash ~/.local/bin/aws`)
- shorthand aliases are also available after sourcing: `aw`, `awc`, `awl`, `awe`, `awr`, `awm`, `awt` (and `awa*` for auth)

Without cloning (zsh):

```sh
mkdir -p "$HOME/.config/agent-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/agent-workspace-launcher/main/scripts/aws.zsh \
  -o "$HOME/.config/agent-workspace-launcher/aws.zsh"
source "$HOME/.config/agent-workspace-launcher/aws.zsh"
```

Without cloning (bash):

```sh
mkdir -p "$HOME/.config/agent-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/agent-workspace-launcher/main/scripts/aws.bash \
  -o "$HOME/.config/agent-workspace-launcher/aws.bash"
source "$HOME/.config/agent-workspace-launcher/aws.bash"
```

Configure wrapper defaults (optional):

```sh
AWS_DOCKER_ARGS=(
  -e HOME="$HOME"
  -v "$HOME/.config:$HOME/.config:ro"
)
AWS_IMAGE="graysurf/agent-workspace-launcher:latest"
```

Create a workspace:

```sh
aws create OWNER/REPO
```

Common operations:

```sh
aws --help
aws ls
aws auth github <name|container>
aws exec <name|container>
aws rm <name|container> --yes
aws rm --all --yes
```

Direct `docker run` is also supported:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/REPO
```

## Private repos (GitHub)

If `gh` is logged in on the host, `aws create/reset/auth github` can reuse that keyring token when
`GH_TOKEN`/`GITHUB_TOKEN` are not set.

Or pass a token explicitly:

```sh
export GH_TOKEN=...
aws create OWNER/PRIVATE_REPO
```

Security note: `create` persists `GH_TOKEN`/`GITHUB_TOKEN` into workspace container env (visible via `docker inspect`).
Treat workspace containers as sensitive.

## DooD host-path rules

- Mounting `docker.sock` is root-equivalent host access.
- Any `-v <src>:<dst>` executed by the launcher resolves `<src>` on the host.
- For host file reads, prefer same-path binds plus `HOME` passthrough.

Example (`~/.config` snapshot):

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config:$HOME/.config:ro" \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/REPO
```

## Optional host mounts

Mount Codex secrets/profile material if your workflow needs host-side profile sync:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config/AGENT_secrets:$HOME/.config/AGENT_secrets:rw" \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/REPO
```

## Configuration

### Host wrapper env (`aws`)

| Env | Default | Purpose |
| --- | --- | --- |
| `AWS_IMAGE` | `graysurf/agent-workspace-launcher:latest` | Launcher image tag |
| `AWS_DOCKER_ARGS` | (empty) | Extra `docker run` args for the launcher container |
| `AWS_AUTH` | `auto` | `auto\|env\|none`; token source policy for GitHub auth flows |

### Launcher env (`agent-workspace`)

| Env | Default | Purpose |
| --- | --- | --- |
| `AGENT_WORKSPACE_PREFIX` | `agent-ws` | Workspace container name prefix |
| `AGENT_WORKSPACE_PRIVATE_REPO` | (empty) | During `create`, clone/pull into `~/.private` |
| `AGENT_WORKSPACE_LAUNCHER` | auto-detect (`/opt/agent-kit/docker/agent-env/bin/agent-workspace`) | Low-level launcher path override |
| `AGENT_WORKSPACE_AUTH` | `auto` | Auth source policy |
| `AGENT_WORKSPACE_GPG` | `none` | Default GPG import mode (`import\|none`) |
| `AGENT_WORKSPACE_GPG_KEY` | (empty) | Default signing key for `auth gpg` |
| `AGENT_WORKSPACE_TUNNEL_NAME` | (empty) | Tunnel name for `tunnel` |

Also used:

- `GH_TOKEN` / `GITHUB_TOKEN`
- `XDG_CACHE_HOME`
- `TMPDIR`

### Codex auth naming exception

For compatibility, Codex auth paths remain:

- `CODEX_SECRET_DIR`
- `CODEX_AUTH_FILE`

These names are intentionally **not** renamed to `AWS_*`.

## Troubleshooting

- Docker daemon not running: verify `docker info`.
- Linux socket permission errors: try `--user 0:0` or `--group-add ...`.
- `exec` with `--` as a separator may be treated as a literal argument; run without `--`.

## Development

- Local builds: [docs/BUILD.md](docs/BUILD.md)
- Architecture notes: [docs/DESIGN.md](docs/DESIGN.md)
- User guides: [docs/guides/README.md](docs/guides/README.md)
- Integration runbook: [docs/runbooks/INTEGRATION_TEST.md](docs/runbooks/INTEGRATION_TEST.md)

Publishing:

- Workflow: [`.github/workflows/publish.yml`](.github/workflows/publish.yml)
- Publish branch: `docker`
- Registries: Docker Hub + GHCR
- Tags: `latest`, `sha-<short>`

## License

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

This project is licensed under the MIT License. See [LICENSE](LICENSE).
