# Install `agent-workspace-launcher`

Primary command: `agent-workspace-launcher`

Compatibility alias: `awl`

Runtime model:

- Supports `container` and `host`
- Default runtime is `container`
- Select runtime with `--runtime container|host` or `AGENT_WORKSPACE_RUNTIME`

## Homebrew (recommended)

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

## Docker Hub (optional launcher-in-container / DooD mode)

This mode is Docker-outside-of-Docker (DooD): `awl_docker` runs the launcher inside
a container that talks to your host Docker daemon via `/var/run/docker.sock`.
It is optional and separate from the built-in `container` runtime (the default for
`agent-workspace-launcher` itself).

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

- `source scripts/awl_docker.zsh` (zsh) or `source scripts/awl_docker.bash` (bash)
  will register completion automatically.

## Build from source

```sh
cargo build --release -p agent-workspace
./target/release/agent-workspace-launcher --help
```

Create `awl` alias (optional):

```sh
ln -sf "$(pwd)/target/release/agent-workspace-launcher" "$HOME/.local/bin/awl"
awl --help
```

## Notes

- `awl` is alias compatibility only.
- `agent-workspace-launcher` is the canonical command name.
- If Docker is unavailable on your host, use `--runtime host` (or set `AGENT_WORKSPACE_RUNTIME=host`).
