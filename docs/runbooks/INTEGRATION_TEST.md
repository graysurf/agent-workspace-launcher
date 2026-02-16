# Integration test checklist

This repo’s “launcher image” is **Docker-outside-of-Docker (DooD)**: the launcher container uses the host Docker
daemon (`/var/run/docker.sock`) to create **workspace containers**.

This checklist is for validating the end-to-end experience after merging to `main`.

## What to verify

- [x] macOS quickstart smoke (zsh wrapper via `cws`; no local build)
- [x] macOS quickstart smoke (bash wrapper via `cws`; no local build)
- [x] Linux exploratory smoke run (captures logs)
- [x] CI publish run URL recorded (on `main`)
- [x] Docker Hub tags exist (`latest`, `sha-<short>`)
- [x] Docker Hub image is multi-arch (`linux/amd64`, `linux/arm64`)
- [x] GHCR tags exist (`latest`, `sha-<short>`)
- [x] GHCR image is multi-arch (`linux/amd64`, `linux/arm64`)

## macOS quickstart smoke (published images; no local build)

This validates the end-user “Quickstart” path:

- Pull + run `graysurf/agent-workspace-launcher`
- During `create`, the workspace runtime image `graysurf/agent-env:linuxbrew` also gets pulled (so you implicitly
  validate it exists and is runnable on your platform).

Pre-flight:

```sh
docker info >/dev/null
```

zsh wrapper:

```sh
source ./scripts/cws.zsh

# Pulls the launcher image and prints help.
cws --help

# Verifies the launcher container can talk to the host daemon.
cws ls

# End-to-end create (public repo).
cws create graysurf/agent-kit

# Copy the printed workspace name, then:
cws exec <name|container> git -C /work/graysurf/agent-kit status
cws rm <name|container> --yes
```

bash wrapper (run in a separate bash shell to avoid mixing wrappers):

```sh
source ./scripts/cws.bash
cws --help
```

Capture evidence:

- Save the full terminal output to a log file and attach it to the integration testing PR (or paste it in a PR comment).

Evidence (2026-01-20):

- `$AGENT_HOME/out/macos-quickstart-smoke-20260120-080236.log`
- `$AGENT_HOME/out/macos-quickstart-create-20260120-080236.log`

## Linux exploratory smoke (do not claim support yet)

Run on a real Linux host with Docker (rootful):

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

- `permission denied` when accessing `/var/run/docker.sock`. Workaround: run as root (`--user 0:0`) or add the
  docker socket group GID via `--group-add ...`.

Capture evidence:

- Save the full terminal output to a log file and attach it to the integration testing PR (or paste it in a PR comment).

Evidence (2026-01-20; OrbStack on macOS):

- `$AGENT_HOME/out/linux-exploratory-smoke-orbstack-20260120-085812.log`
- `$AGENT_HOME/out/linux-exploratory-create-orbstack-20260120-085812.log`

## CI publish verification

After merge to `main`, verify:

- GitHub Actions workflow `.github/workflows/publish.yml` ran successfully on `main` (record the run URL).
- Docker Hub has the expected tags:
  - `graysurf/agent-workspace-launcher:latest`
  - `graysurf/agent-workspace-launcher:sha-<short>`
- GHCR has the expected tags:
  - `ghcr.io/graysurf/agent-workspace-launcher:latest`
  - `ghcr.io/graysurf/agent-workspace-launcher:sha-<short>`
- The published images are multi-arch:

```sh
docker buildx imagetools inspect graysurf/agent-workspace-launcher:latest
docker buildx imagetools inspect ghcr.io/graysurf/agent-workspace-launcher:latest
```

Expected platforms include `linux/amd64` and `linux/arm64`.

Evidence (2026-01-20):

- Docker Hub verification (before GHCR publish): https://github.com/graysurf/agent-workspace-launcher/actions/runs/21154177325
- Docker Hub inspect log: `$AGENT_HOME/out/ci-publish-verification-20260120-081548.log`
- PR build (no publish): https://github.com/graysurf/agent-workspace-launcher/actions/runs/21155245507
- GHCR verification (push to `main`): https://github.com/graysurf/agent-workspace-launcher/actions/runs/21155498181
- GHCR inspect log: `$AGENT_HOME/out/ghcr-verification-20260120-084948.log`
- GHCR not found (pre-merge check): `$AGENT_HOME/out/ghcr-verification-20260120-083359.log`
