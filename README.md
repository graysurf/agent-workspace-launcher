# agent-workspace-launcher

Host-native workspace lifecycle CLI for repository-focused development.

- Primary command: `agent-workspace-launcher`
- Compatibility alias: `awl` (via shell wrapper or symlink)
- Host-native usage is the primary path; Docker image usage is optional
- Subcommands: `auth`, `create`, `ls`, `rm`, `exec`, `reset`, `tunnel`

## Requirements

- `git` (required)
- Optional for specific flows:
  - `gh` (GitHub token/keyring auth)
  - `gpg` (signing key checks)
  - `code` (VS Code tunnel)

## Install

### Homebrew

```sh
brew tap graysurf/tap
brew install agent-workspace-launcher
agent-workspace-launcher --help
awl --help
```

Homebrew completion (default via formula):

```sh
agent-workspace-launcher <TAB>
awl <TAB>
```

Upgrade to latest tap release and verify both commands:

```sh
brew update-reset "$(brew --repo graysurf/tap)"
brew upgrade graysurf/tap/agent-workspace-launcher || brew install graysurf/tap/agent-workspace-launcher
agent-workspace-launcher --version
awl --version
```

### Docker Hub (DooD, no brew required)

This mode is Docker-outside-of-Docker (DooD): `awl_docker` runs inside a launcher container
that talks to your host Docker daemon via `/var/run/docker.sock`.

```sh
docker pull graysurf/agent-workspace-launcher:latest
docker pull graysurf/agent-env:latest
```

Without cloning (zsh):

```sh
mkdir -p "$HOME/.config/agent-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/agent-workspace-launcher/main/scripts/awl_docker.zsh \
  -o "$HOME/.config/agent-workspace-launcher/awl_docker.zsh"
autoload -Uz compinit
compinit
source "$HOME/.config/agent-workspace-launcher/awl_docker.zsh"
```

Without cloning (bash):

```sh
mkdir -p "$HOME/.config/agent-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/agent-workspace-launcher/main/scripts/awl_docker.bash \
  -o "$HOME/.config/agent-workspace-launcher/awl_docker.bash"
source "$HOME/.config/agent-workspace-launcher/awl_docker.bash"
```

Configure wrapper defaults (optional):

zsh:

```sh
AWL_DOCKER_IMAGE="graysurf/agent-workspace-launcher:latest"
AWL_DOCKER_AGENT_ENV_IMAGE="graysurf/agent-env:latest"
AWL_DOCKER_ARGS=(
  -e HOME="$HOME"
  -v "$HOME/.config:$HOME/.config:ro"
)
```

bash:

```sh
AWL_DOCKER_IMAGE="graysurf/agent-workspace-launcher:latest"
AWL_DOCKER_AGENT_ENV_IMAGE="graysurf/agent-env:latest"
AWL_DOCKER_ARGS="-e HOME=$HOME -v $HOME/.config:$HOME/.config:ro"
```

Common operations:

```sh
awl_docker --help
awl_docker create OWNER/REPO
awl_docker ls
awl_docker exec <workspace>
awl_docker rm <workspace> --yes
```

Direct `docker run` is also supported:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e AGENT_ENV_IMAGE=graysurf/agent-env:latest \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/REPO
```

DooD host-path rules:

- Mounting `docker.sock` is root-equivalent host access.
- Any `-v <src>:<dst>` sent by launcher resolves `<src>` on the host.
- If you need host files (`~/.config`, `~/.ssh`), pass same-path mounts explicitly.

`awl_docker` completion:

- `source scripts/awl_docker.zsh` (zsh) or `source scripts/awl_docker.bash` (bash) will register completion automatically.

### Build from source

```sh
cargo build --release -p agent-workspace
./target/release/agent-workspace-launcher --help
```

Create `awl` alias (optional):

```sh
ln -sf "$(pwd)/target/release/agent-workspace-launcher" "$HOME/.local/bin/awl"
awl --help
```

## Quickstart

Create and use a workspace:

```sh
agent-workspace-launcher create OWNER/REPO
agent-workspace-launcher ls
agent-workspace-launcher exec <workspace>
agent-workspace-launcher rm <workspace> --yes
```

## Workspace storage

Default root:

- `AGENT_WORKSPACE_HOME` (if set)
- else `XDG_STATE_HOME/agent-workspace-launcher/workspaces`
- else `$HOME/.local/state/agent-workspace-launcher/workspaces`

## Command notes

- `create`: makes a host workspace directory and optionally clones repo(s).
- `exec`: runs command or login shell from workspace path.
- `reset`: host-side git reset flows (`repo`, `work-repos`, `opt-repos`, `private-repo`).
- `auth github`: stores resolved host token under workspace auth directory.
- `auth codex`: syncs Codex auth files while keeping compatibility names.
- `tunnel`: runs `code tunnel` from workspace path.

## Environment variables

| Env | Default | Purpose |
| --- | --- | --- |
| `AGENT_WORKSPACE_HOME` | auto | Workspace root override |
| `AGENT_WORKSPACE_PREFIX` | `agent-ws` | Prefix normalization for workspace names |
| `AGENT_WORKSPACE_AUTH` | `auto` | GitHub auth token policy: `auto|gh|env|none` |
| `AGENT_WORKSPACE_GPG_KEY` | (empty) | Default key for `auth gpg` |
| `CODEX_SECRET_DIR` | (empty) | Codex profile directory (compatibility name) |
| `CODEX_AUTH_FILE` | `~/.codex/auth.json` | Codex auth file path (compatibility name) |

## Alias wrappers

- `scripts/awl.bash`
- `scripts/awl.zsh`
- `scripts/awl_docker.bash`
- `scripts/awl_docker.zsh`

These wrappers call `agent-workspace-launcher` directly and expose `aw*` shortcuts.

## Development

- Build/test guide: `docs/BUILD.md`
- Architecture: `docs/DESIGN.md`
- User guide: `docs/guides/awl/README.md`
- Release guide: `docs/RELEASE_GUIDE.md`

## License

MIT. See `LICENSE`.
