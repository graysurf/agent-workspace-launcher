# Integration test checklist

This repo’s “launcher image” is **Docker-outside-of-Docker (DooD)**: the launcher container uses the host Docker
daemon (`/var/run/docker.sock`) to create **workspace containers**.

This checklist is for validating the end-to-end experience after merging to `main`.

## What to verify

- [ ] macOS quickstart smoke (zsh wrapper via `cws`; no local build)
- [ ] macOS quickstart smoke (bash wrapper via `cws`; no local build)
- [ ] Linux exploratory smoke run (captures logs)
- [ ] CI publish run URL recorded (on `main`)
- [ ] Published Docker Hub tags exist (`latest`, `sha-<short>`)
- [ ] Published image is multi-arch (`linux/amd64`, `linux/arm64`)

## macOS quickstart smoke (published images; no local build)

This validates the end-user “Quickstart” path:

- Pull + run `graysurf/codex-workspace-launcher`
- During `create`, the workspace runtime image `graysurf/codex-env:linuxbrew` also gets pulled (so you implicitly
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
cws create graysurf/codex-kit

# Copy the printed workspace name, then:
cws exec <name|container> git -C /work/graysurf/codex-kit status
cws rm <name|container> --yes
```

bash wrapper (run in a separate bash shell to avoid mixing wrappers):

```sh
source ./scripts/cws.bash
cws --help
```

Capture evidence:

- Save the full terminal output to a log file and attach it to the integration testing PR (or paste it in a PR comment).

## Linux exploratory smoke (do not claim support yet)

Run on a real Linux host with Docker (rootful):

```sh
# Should print help without talking to the Docker daemon.
docker run --rm -it graysurf/codex-workspace-launcher:latest --help

# Verify Docker daemon connectivity from the launcher container.
docker run --rm -it \
  --user 0:0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/codex-workspace-launcher:latest \
  ls

# End-to-end create (public repo).
docker run --rm -it \
  --user 0:0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/codex-workspace-launcher:latest \
  create graysurf/codex-kit
```

Expected failure mode:

- `permission denied` when accessing `/var/run/docker.sock`. Workaround: run as root (`--user 0:0`) or add the
  docker socket group GID via `--group-add ...`.

Capture evidence:

- Save the full terminal output to a log file and attach it to the integration testing PR (or paste it in a PR comment).

## CI publish verification

After merge to `main`, verify:

- GitHub Actions workflow `.github/workflows/publish.yml` ran successfully on `main` (record the run URL).
- Docker Hub has the expected tags:
  - `graysurf/codex-workspace-launcher:latest`
  - `graysurf/codex-workspace-launcher:sha-<short>`
- The published image is multi-arch:

```sh
docker buildx imagetools inspect graysurf/codex-workspace-launcher:latest
```

Expected platforms include `linux/amd64` and `linux/arm64`.
