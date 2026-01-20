# Install `cws` (zsh / bash)

`cws` is optional, but recommended. It saves you from re-typing `docker run ...` and includes shell completion.

## Option A: clone the repo (recommended for contributors)

```sh
git clone https://github.com/graysurf/codex-workspace-launcher.git
cd codex-workspace-launcher

# zsh
source ./scripts/cws.zsh

# bash
# source ./scripts/cws.bash
```

## Option B: without cloning (recommended for users)

zsh:

```sh
mkdir -p "$HOME/.config/codex-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/codex-workspace-launcher/main/scripts/cws.zsh \
  -o "$HOME/.config/codex-workspace-launcher/cws.zsh"
source "$HOME/.config/codex-workspace-launcher/cws.zsh"
```

bash:

```sh
mkdir -p "$HOME/.config/codex-workspace-launcher"
curl -fsSL https://raw.githubusercontent.com/graysurf/codex-workspace-launcher/main/scripts/cws.bash \
  -o "$HOME/.config/codex-workspace-launcher/cws.bash"
source "$HOME/.config/codex-workspace-launcher/cws.bash"
```

## Option C: install as an executable

```sh
cp ./scripts/cws ~/.local/bin/cws
```

## Optional: shell completion

- zsh: completion registers automatically if `compinit` is enabled.
- bash: completion registers when you `source ./scripts/cws.bash`.

## Configuration

### Choose the launcher image

Default:

```sh
export CWS_IMAGE="graysurf/codex-workspace-launcher:latest"
```

Use GHCR:

```sh
export CWS_IMAGE="ghcr.io/graysurf/codex-workspace-launcher:latest"
```

### Add extra `docker run` args (optional)

Examples:

- Pass your host `HOME` into the launcher container
- Bind-mount host `~/.config` into the launcher (same-path bind; DooD-safe)

zsh (array form preserves quoting):

```sh
CWS_DOCKER_ARGS=(
  -e HOME="$HOME"
  -v "$HOME/.config:$HOME/.config:ro"
)
```

bash (string form is simplest):

```sh
export CWS_DOCKER_ARGS="-e HOME=$HOME -v $HOME/.config:$HOME/.config:ro"
```

## Private repos

If you have `gh` logged in on the host, `cws create/reset/auth github` will automatically reuse that token
(keyring) when `GH_TOKEN`/`GITHUB_TOKEN` are not set.

Or set `GH_TOKEN` (or `GITHUB_TOKEN`) on your host — `cws` forwards it into the launcher container:

```sh
export GH_TOKEN=...
cws create OWNER/PRIVATE_REPO
```
