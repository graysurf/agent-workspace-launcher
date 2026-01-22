# Build guide (custom image tags)

This guide is for users who want to **build the launcher image locally**, tag it, and point `cws` at that tag.

Notes:

- Primary target is macOS (Docker Desktop / OrbStack). Linux/WSL can work but may need extra Docker socket permissions.
- The launcher image uses Docker-outside-of-Docker (DooD): it needs access to the host Docker daemon via `/var/run/docker.sock`.

## Requirements

- Docker is installed and running (`docker info` works).
- Git (to clone this repo).

## Build (simple)

Clone and build a local tag:

```sh
git clone https://github.com/graysurf/codex-workspace-launcher.git
cd codex-workspace-launcher

docker build -t codex-workspace-launcher:local .
```

Verify the image works:

```sh
docker run --rm -it codex-workspace-launcher:local --help
```

## Use with `cws`

If you cloned the repo, you can source the wrapper and point it at your local image tag:

zsh:

```sh
source ./scripts/cws.zsh
export CWS_IMAGE="codex-workspace-launcher:local"

cws --help
cws create OWNER/REPO
```

bash:

```sh
source ./scripts/cws.bash
export CWS_IMAGE="codex-workspace-launcher:local"

cws --help
cws create OWNER/REPO
```

Executable (any shell; no completion):

```sh
export CWS_IMAGE="codex-workspace-launcher:local"
./scripts/cws.bash --help
./scripts/cws.bash create OWNER/REPO
```

You can also override the image per-command:

```sh
CWS_IMAGE="codex-workspace-launcher:local" cws ls
```

## Pin upstream refs (recommended for reproducibility)

The Dockerfile clones `zsh-kit` and `codex-kit` at build time. You can pin them to a branch, tag, or commit SHA:

```sh
docker build -t codex-workspace-launcher:local \
  --build-arg ZSH_KIT_REF=main \
  --build-arg CODEX_KIT_REF=main \
  .
```

For reproducible builds, prefer commit SHAs:

```sh
docker build -t codex-workspace-launcher:local \
  --build-arg ZSH_KIT_REF=<zsh-kit-sha> \
  --build-arg CODEX_KIT_REF=<codex-kit-sha> \
  .
```

This repo also ships a pinned pair in `VERSIONS.env` (used by CI). Build the exact pins like this:

```sh
set -euo pipefail
set -a
source ./VERSIONS.env
set +a

docker build -t codex-workspace-launcher:local \
  --build-arg ZSH_KIT_REF="$ZSH_KIT_REF" \
  --build-arg CODEX_KIT_REF="$CODEX_KIT_REF" \
  .
```

## Build from forks / alternate sources

You can build against a fork by overriding the repo URLs:

```sh
docker build -t codex-workspace-launcher:local \
  --build-arg ZSH_KIT_REPO="https://github.com/<you>/zsh-kit.git" \
  --build-arg ZSH_KIT_REF="main" \
  --build-arg CODEX_KIT_REPO="https://github.com/<you>/codex-kit.git" \
  --build-arg CODEX_KIT_REF="main" \
  .
```

Private forks:

- The default Dockerfile clone uses HTTPS without credentials.
- If your fork is private, you’ll need to make it public, or extend the build to provide git credentials (BuildKit secrets, SSH, etc.).

## Advanced: cross-platform builds (buildx)

`docker build` builds for your current architecture. If you need a different platform (or multi-arch), use `buildx`.

Example: build an amd64 image on Apple Silicon and load it locally:

```sh
docker buildx build \
  --platform linux/amd64 \
  -t codex-workspace-launcher:local-amd64 \
  --load \
  .
```

Example: build both amd64+arm64 and push to a registry (requires login):

```sh
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t <your-registry>/codex-workspace-launcher:local \
  --push \
  .
```

## Linux / WSL notes

- Linux: you may see `permission denied` on `/var/run/docker.sock`.
  - Quick workaround: add `--user 0:0` via `CWS_DOCKER_ARGS`.
  - Alternative: add the docker group GID via `--group-add ...`.
- WSL2: ensure Docker Desktop WSL integration is enabled and `docker info` works inside your distro.

Example (`--user 0:0`):

For the sourced `cws` function (zsh/bash):

```sh
CWS_DOCKER_ARGS=(--user 0:0)
```

For the executable `cws` script (string form):

```sh
export CWS_DOCKER_ARGS="--user 0:0"
```
