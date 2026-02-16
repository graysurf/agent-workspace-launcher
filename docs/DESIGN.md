# Design: agent-workspace-launcher

Goal: package `zsh-kit`'s `agent-workspace` (`auth/create/ls/rm/exec/reset/tunnel`) into a Docker image so a macOS
machine without `zsh-kit` + `agent-kit` checked out can still use it with Docker only:

```sh
docker run ... graysurf/agent-workspace-launcher create OWNER/REPO
```

Platform: macOS (Docker Desktop / OrbStack).

---

## Local References (Source of Truth)

This repo is extracted from the existing local setup to create a portable launcher. Keep these paths at the
top of the doc for quick lookups during development:

- Project repo (local): `/Users/terry/Project/graysurf/agent-workspace-launcher`
- `agent-kit` (local): `/Users/terry/.config/agent-kit`
  - Workspace runtime image (Ubuntu 24.04 + tools): `/Users/terry/.config/agent-kit/Dockerfile`
  - Low-level launcher (bash; subcommands are `up/...`): `/Users/terry/.config/agent-kit/docker/agent-env/bin/agent-workspace`
- `zsh-kit` (local): `/Users/terry/.config/zsh`
  - Full feature set packaged by this repo (zsh; subcommands are `create/...`): `/Users/terry/.config/zsh/scripts/_features/agent-workspace/workspace-launcher.zsh`

---

## Architecture in One Sentence (Two Layers)

1) This repo builds a **launcher image** that exposes `agent-workspace create ...` (the public interface is `create`).
2) The launcher uses `docker.sock` to create/manage **workspace containers** on the host (runtime image defaults to `graysurf/agent-env:linuxbrew`).

---

## Quickstart (Minimal: No Host Config Mounts)

Requirements:

- Docker Desktop / OrbStack running on macOS
- `docker info` succeeds

Public repo:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/agent-workspace-launcher:latest \
  create graysurf/agent-kit
```

Private repo (token only; no secrets/config mounts by default):

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN="$GH_TOKEN" \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/PRIVATE_REPO
```

Common operations:

```sh
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/agent-workspace-launcher:latest ls
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/agent-workspace-launcher:latest exec <name|container>
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/agent-workspace-launcher:latest rm <name|container> --yes
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/agent-workspace-launcher:latest rm --all --yes
```

---

## DooD Rules (Avoid Footguns)

This launcher is **Docker outside of Docker (DooD)**: the `docker` CLI inside the launcher container talks to the
host Docker daemon via `-v /var/run/docker.sock:/var/run/docker.sock`.

Consequence: any `docker run -v <src>:<dst>` executed by the launcher container:

- Resolves `<src>` on the **host (macOS)**, not inside the launcher container
- Therefore `<src>` must be an **absolute path that exists on the host** (recommend using `$HOME/...` paths)

Therefore, any feature that reads host files (secrets / `~/.config` snapshot / private repo seed) must satisfy:

1) The zsh/bash layer receives a **host absolute path** (recommend using `$HOME/...`)
2) The launcher container can also `test -d` that path (otherwise it will be treated as missing and skipped)

The simplest approach is to bind-mount `host_path:host_path` into the launcher container using the same absolute
path, and set `-e HOME="$HOME"` so `$HOME` expands consistently inside the launcher container.

Example: mount host `~/.config/AGENT_secrets` (read-write; compatible with `codex-use` syncing)

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config/AGENT_secrets:$HOME/.config/AGENT_secrets:rw" \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/REPO
```

Example: enable host `~/.config` snapshot (the launcher copies into the workspace; not a bind mount)

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config:$HOME/.config:ro" \
  graysurf/agent-workspace-launcher:latest \
  create OWNER/REPO
```

---

## Environment Variables (Must Be Documented and Configurable)

### A) zsh layer: full-feature entrypoint (`create/...`)

Source (local): `/Users/terry/.config/zsh/scripts/_features/agent-workspace/workspace-launcher.zsh`

| Env | Default | Purpose |
| --- | --- | --- |
| `AGENT_WORKSPACE_PREFIX` | `codex-ws` | Workspace container name prefix |
| `AGENT_WORKSPACE_PRIVATE_REPO` | (empty) | During `create`, clone/pull this repo into workspace `~/.private` |
| `AGENT_WORKSPACE_LAUNCHER` | (empty) | Path to the low-level launcher script (recommended default in this image: `/opt/agent-kit/docker/agent-env/bin/agent-workspace`) |
| `AGENT_WORKSPACE_LAUNCHER_AUTO_DOWNLOAD` | `true` | Auto-download launcher when missing (recommended: `false` in this image, or set `AGENT_WORKSPACE_LAUNCHER` to avoid runtime network dependency) |
| `AGENT_WORKSPACE_LAUNCHER_URL` | GitHub raw | Launcher download URL (only when auto-download is enabled and the script is missing) |
| `AGENT_WORKSPACE_LAUNCHER_AUTO_PATH` | `$XDG_CACHE_HOME/...` | Auto-download install path |
| `AGENT_WORKSPACE_AUTH` | `auto` | `auto\|gh\|env\|none`: token source selection (inside the launcher container, `env` is usually the most practical) |
| `AGENT_WORKSPACE_GPG` | `none` | Default gpg import mode for `create` (`import\|none`) |
| `AGENT_WORKSPACE_GPG_KEY` | (empty) | Default signing key for `auth gpg` (keyid or fingerprint) |
| `AGENT_WORKSPACE_TUNNEL_NAME` | (empty) | Tunnel name for the `tunnel` subcommand (<= 20 chars) |
| `AGENT_WORKSPACE_OPEN_VSCODE_ENABLED` | (empty/false) | Auto-run `code --new-window` if host has `code` CLI (typically not effective inside the launcher container) |

Additional variables used (without `AGENT_WORKSPACE_` prefix, but behavior depends on them):

- `GH_TOKEN` / `GITHUB_TOKEN`: clone private repos; configure git auth inside the workspace
- `XDG_CACHE_HOME`: launcher auto-download cache root
- `TMPDIR`: temp files

### B) low-level launcher (invoked by the zsh layer; `up/...`)

Source (local): `/Users/terry/.config/agent-kit/docker/agent-env/bin/agent-workspace`

| Env | Default | Purpose |
| --- | --- | --- |
| `AGENT_ENV_IMAGE` | `graysurf/agent-env:linuxbrew` | Workspace runtime image |
| `AGENT_WORKSPACE_PREFIX` | `codex-ws` | Same as above (container/volume naming) |
| `GITHUB_HOST` | `github.com` | Repo host (when using `OWNER/REPO` form) |
| `DEFAULT_SECRETS_MOUNT` | `/home/agent/AGENT_secrets` | Default container mount path when `--secrets-dir` is used |

---

## Work Plan (Standalone Repo: Implementation Steps)

### Step 0: define the external contract (DoD)

  - `docker run ... <image> --help` shows `auth/create/ls/rm/exec/reset/tunnel` (aligned with `workspace-launcher.zsh` usage).
- `create OWNER/REPO` works on a clean macOS machine (Docker only) and creates a workspace container (named volumes).
- No host mounts required by default (secrets/config/private are optional).
- When `GH_TOKEN` is present, cloning private repos works.

### Step 1: build the launcher image (Dockerfile + wrapper)

Recommended minimal layout:

```text
Dockerfile
bin/agent-workspace        # bundled entrypoint: vendored zsh-kit `agent-workspace`
.github/workflows/publish.yml
```

Dockerfile design notes (macOS host; Ubuntu base recommended for predictable tooling):

- Install: `docker` CLI, `zsh`, `git`, `curl`, `ca-certificates`, `tar`, `rsync`, `python3`, `openssl`,
  and standard `awk/sed/coreutils`
- Clone/pin:
  - `/opt/agent-kit` (contains `docker/agent-env/bin/agent-workspace`)
- Record zsh-kit ref (for traceability): `/opt/zsh-kit.ref`
- Set defaults: `ENV AGENT_WORKSPACE_LAUNCHER=/opt/agent-kit/docker/agent-env/bin/agent-workspace`
- Set `ENTRYPOINT ["agent-workspace"]`

Local build (start with a dev tag):

```sh
docker build -t agent-workspace-launcher:dev \
  --build-arg ZSH_KIT_REF=main \
  --build-arg AGENT_KIT_REF=main \
  .
```

### Step 2: expose `create` as a container-executable command

Wrapper requirements:

1) `bin/agent-workspace` is a bundled wrapper generated from `zsh-kit` and committed to this repo
2) `docker run ... <image> <args...>` executes the wrapper (image entrypoint)

Verification:

```sh
docker run --rm -it agent-workspace-launcher:dev --help
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock agent-workspace-launcher:dev ls
```

### Step 3: minimal smoke checks (validate end-to-end)

```sh
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock agent-workspace-launcher:dev create graysurf/agent-kit
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock agent-workspace-launcher:dev ls
```

### Step 4: CI publishing (GitHub Actions)

Recommendation (macOS users can be Apple Silicon or Intel):

- Multi-arch: `linux/amd64,linux/arm64`
- Tags: `latest` + `sha-<short>` (or semver)
- Build args: `ZSH_KIT_REF`, `AGENT_KIT_REF` (use commit SHAs for reproducibility)
- Source of truth: `VERSIONS.env` (CI should build from the pinned pair)

Example (manual):

```sh
docker buildx build --platform linux/amd64,linux/arm64 \
  -t graysurf/agent-workspace-launcher:latest \
  -t graysurf/agent-workspace-launcher:sha-$(git rev-parse --short HEAD) \
  --build-arg ZSH_KIT_REF=main \
  --build-arg AGENT_KIT_REF=main \
  --push \
  .
```

### Step 5: required documentation sections (non-negotiable)

- Quickstart (docker.sock only)
- Private repos (`GH_TOKEN`)
- DooD host-path rules (include secrets/config snapshot examples)
- Complete env/flags table
- Security notes (token persistence, docker.sock is effectively root)
- Documentation portability: avoid hard-coded `/Users/...` in runnable examples; prefer `$HOME/...` + same-path binds

### Step 6: Linux smoke verification (do not claim support yet)

Goal: validate whether the launcher works on a generic Linux host (rootful Docker) without committing to Linux
support in `README.md`.

Smoke commands (Linux host):

```sh
# Should print help without talking to the Docker daemon.
docker run --rm -it graysurf/agent-workspace-launcher:latest --help

# Verify Docker daemon connectivity from the launcher container.
docker run --rm -it \
  --user 0:0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/agent-workspace-launcher:latest \
  ls

# End-to-end create (public repo).
docker run --rm -it \
  --user 0:0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/agent-workspace-launcher:latest \
  create graysurf/agent-kit
```

Expected failure mode:

- `permission denied` when accessing `/var/run/docker.sock` (common on Linux). Workaround: run the launcher
  container as root (`--user 0:0`) or add the docker socket group GID to the container (`--group-add ...`).

---

## Security / Risks (Must Be Explicit)

- `-v /var/run/docker.sock:/var/run/docker.sock` effectively grants root-equivalent control over the host (can create privileged containers and access mounted paths).
