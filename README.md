# codex-workspace-launcher

Launch a `Codex-ready` workspace for any repo — `prompts`, `skills`, and common CLI tools included.

- Workspace includes `rg`, `gh`, `jq`, `git` (and more) so you can start collaborating with Codex immediately
- VS Code friendly: Dev Containers attach + optional VS Code tunnel
- Optional `cws` wrapper (zsh + bash, with completion) so you don’t repeat `docker run ...`
- Under the hood: powered by [zsh-kit](https://github.com/graysurf/zsh-kit) (bundled into `bin/codex-workspace`) and [codex-kit](https://github.com/graysurf/codex-kit) (vendored into the image; published images pin SHAs)

This project packages the `codex-workspace` CLI (`auth/create/ls/rm/exec/reset/tunnel`) as a Docker image — no local
setup required.

This is **Docker-outside-of-Docker (DooD)**: the launcher container talks to your host Docker daemon via
`/var/run/docker.sock` and creates **workspace containers** on the host (default runtime image:
`graysurf/codex-env:linuxbrew`).

## Requirements

- Docker Desktop / OrbStack (macOS). Linux may work but is not fully smoke-tested yet.
- You can mount the Docker socket: `-v /var/run/docker.sock:/var/run/docker.sock`

## Quickstart

Use the provided `cws` wrapper (recommended):

- zsh: `source ./scripts/cws.zsh` (completion registers once `compinit` is available; see [`scripts/cws.zsh`](scripts/cws.zsh))
- bash: `source ./scripts/cws.bash` (see [`scripts/cws.bash`](scripts/cws.bash))
- executable: put `./scripts/cws.bash` on your `PATH` (example: `cp ./scripts/cws.bash ~/.local/bin/cws`; see [`scripts/cws.bash`](scripts/cws.bash))

Without cloning (zsh):

```sh
mkdir -p "$HOME/.config/codex-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/codex-workspace-launcher/main/scripts/cws.zsh \
  -o "$HOME/.config/codex-workspace-launcher/cws.zsh"
source "$HOME/.config/codex-workspace-launcher/cws.zsh"
```

Without cloning (bash):

```sh
mkdir -p "$HOME/.config/codex-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/codex-workspace-launcher/main/scripts/cws.bash \
  -o "$HOME/.config/codex-workspace-launcher/cws.bash"
source "$HOME/.config/codex-workspace-launcher/cws.bash"
```

Customize defaults (optional):

```sh
# Extra docker-run args (zsh/bash array form; preserves quoting)
CWS_DOCKER_ARGS=(
  -e HOME="$HOME"
  -v "$HOME/.config:$HOME/.config:ro"
)

# Override the image tag
CWS_IMAGE="graysurf/codex-workspace-launcher:latest"
```

Want to build locally and use a custom image tag? See [`docs/BUILD.md`](docs/BUILD.md).

Create a workspace (public repo):

```sh
cws create OWNER/REPO
```

The `create` output prints:

- `workspace: <container>`
- `path: /work/<owner>/<repo>`

Common operations:

```sh
cws --help
cws ls
cws auth github <name|container>
cws exec <name|container>
cws rm <name|container> --yes
cws rm --all --yes
```

Note: you can also define your own small wrapper instead of sourcing scripts, e.g.
`cws(){ docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock -e GH_TOKEN -e GITHUB_TOKEN graysurf/codex-workspace-launcher:latest "$@"; }`

## Working in the workspace

- The repo lives in Docker named volumes (not a host bind mount).
- Use `exec` to enter the container, or attach with VS Code Dev Containers.

Run a command in the workspace:

```sh
cws exec <name|container> git status
```

Interactive shell:

```sh
cws exec <name|container>
```

VS Code Dev Containers:

- `Cmd/Ctrl+Shift+P` → “Dev Containers: Attach to Running Container…” → select the workspace container.

Exec gotcha:

- `codex-workspace exec <name> -- <cmd>` is currently **not supported** (it will try to run `--`).
- Use `codex-workspace exec <name> <cmd...>` instead.

## Private repos (GitHub)

If you have `gh` logged in on the host, `cws create/reset/auth github` will automatically reuse that token
(keyring) when `GH_TOKEN`/`GITHUB_TOKEN` are not set.

Or pass a token into the launcher container:

```sh
export GH_TOKEN=...
cws create OWNER/PRIVATE_REPO
```

Security note: `create` persists `GH_TOKEN`/`GITHUB_TOKEN` into the workspace container environment to make `git`
auth work inside the workspace. Treat the workspace container as sensitive and remove it when done.

## Docker-outside-of-Docker (DooD) rules

- The launcher container talks to the host Docker daemon via `-v /var/run/docker.sock:/var/run/docker.sock`.
- Any `-v <src>:<dst>` executed by the launcher resolves `<src>` on the host.
- When `codex-workspace` needs to read host files, the launcher container must also be able to `test -d` those
  paths (so bind-mount them into the launcher using the same absolute path).

Recommended pattern: same-path binds + `HOME` passthrough.

Example: enable host `~/.config` snapshot (copied into the workspace; not a bind mount):

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config:$HOME/.config:ro" \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

## Optional host mounts

Secrets dir (recommended if you already have it; enables [codex-use](https://github.com/graysurf/zsh-kit/blob/0d48df3ef64fdef3641cfb7caf99be971c3286d8/scripts/_features/codex/_codex-secret.zsh#L396) syncing inside the workspace):

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config/codex_secrets:$HOME/.config/codex_secrets:rw" \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

## Configuration (env vars)

`codex-workspace` (zsh layer; user-facing CLI):

| Env | Default | Purpose |
| --- | --- | --- |
| `CODEX_WORKSPACE_PREFIX` | `codex-ws` | Workspace container name prefix |
| `CODEX_WORKSPACE_PRIVATE_REPO` | (empty) | During `create`, clone/pull this repo into workspace `~/.private` |
| `CODEX_WORKSPACE_LAUNCHER` | (in image) | Low-level launcher path (this image sets it to `/opt/codex-kit/docker/codex-env/bin/codex-workspace`) |
| `CODEX_WORKSPACE_LAUNCHER_AUTO_DOWNLOAD` | `true` | Auto-download low-level launcher when missing (not used when `CODEX_WORKSPACE_LAUNCHER` is set) |
| `CODEX_WORKSPACE_AUTH` | `auto` | `auto\|gh\|env\|none`: token source selection (`env` is most practical in the launcher container) |
| `CODEX_WORKSPACE_GPG` | `none` | Default gpg import mode for `create` (`import\|none`) |
| `CODEX_WORKSPACE_GPG_KEY` | (empty) | Default signing key for `auth gpg` (keyid or fingerprint) |
| `CODEX_WORKSPACE_TUNNEL_NAME` | (empty) | Tunnel name for the `tunnel` subcommand (<= 20 chars) |
| `CODEX_WORKSPACE_OPEN_VSCODE_ENABLED` | (empty/false) | Auto-run `code --new-window` (typically not effective inside the launcher container) |

Additional variables used:

- `GH_TOKEN` / `GITHUB_TOKEN`: clone private repos and configure git auth inside the workspace
- `XDG_CACHE_HOME`: launcher auto-download cache root (only when auto-download is enabled)
- `TMPDIR`: temp files

Low-level launcher (`codex-kit` script; invoked by the zsh layer):

| Env | Default | Purpose |
| --- | --- | --- |
| `CODEX_ENV_IMAGE` | `graysurf/codex-env:linuxbrew` | Workspace runtime image |
| `CODEX_WORKSPACE_PREFIX` | `codex-ws` | Workspace container name prefix |
| `GITHUB_HOST` | `github.com` | Repo host (when using `OWNER/REPO` form) |
| `DEFAULT_SECRETS_MOUNT` | `/home/codex/codex_secrets` | Default container mount path when `--secrets-dir` is used |

## Troubleshooting

- Docker daemon not running: start Docker Desktop/OrbStack; verify `docker info`.
- Linux `permission denied` on `/var/run/docker.sock`: try `--user 0:0` or add the docker socket group GID via
  `--group-add ...`.
- `exec` tries to run `--`: don’t put `--` after the container name.

## Security notes

- Mounting `docker.sock` is root-equivalent host access.
- Persisted `GH_TOKEN`/`GITHUB_TOKEN` values are visible via `docker inspect` on the workspace containers.

## Development

Local builds (custom tags): [`docs/BUILD.md`](docs/BUILD.md)
Upstream pin bumps: [`docs/runbooks/VERSION_BUMPS.md`](docs/runbooks/VERSION_BUMPS.md)

Publishing (CI):

- Workflow: [`.github/workflows/publish.yml`](.github/workflows/publish.yml)
- Triggers: PRs build only; pushes to `docker` publish images
- Registries:
  - Docker Hub: `graysurf/codex-workspace-launcher` (publish requires secrets)
  - GHCR: `ghcr.io/graysurf/codex-workspace-launcher` (publish uses `GITHUB_TOKEN`)
- Tags: `latest`, `sha-<short>`
- Secrets (GitHub Actions; Docker Hub only): `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`
- Ref pinning: `VERSIONS.env` is the single source of truth for pinned `zsh-kit` + `codex-kit` refs.
  - Bump pins via: `docs/runbooks/VERSION_BUMPS.md`

## Docs

- [docs/DESIGN.md](docs/DESIGN.md)
- [docs/BUILD.md](docs/BUILD.md)
- [docs/guides/README.md](docs/guides/README.md)

## 🪪 License

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

This project is licensed under the MIT License. See [LICENSE](LICENSE).
