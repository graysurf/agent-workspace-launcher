---
name: release-docker-image
description: Release agent-workspace-launcher container images to Docker Hub and GHCR using AWL_* env contracts.
---

# Release Docker Image

## Contract

Prereqs:

- Run in repo root (git work tree).
- `docker` with `buildx` enabled.
- `git` available on `PATH`.
- Docker Hub + GHCR credentials provided by `AWL_DOCKER_RELEASE_*` env vars.
- `VERSIONS.env` must contain `AGENT_KIT_REF` (or override with `AWL_DOCKER_RELEASE_AGENT_KIT_REF`).

Inputs:

- CLI:
  - `--version <vX.Y.Z>` (optional but required when `push_version_tag` is enabled)
  - `--ref <git-ref>` (default: `HEAD`)
  - `--dry-run` (render actions without login/build/push)
- Environment (`.env` supported):
  - `AWL_DOCKER_RELEASE_DOCKERHUB_USERNAME`, `AWL_DOCKER_RELEASE_DOCKERHUB_TOKEN`
  - `AWL_DOCKER_RELEASE_GHCR_USERNAME`, `AWL_DOCKER_RELEASE_GHCR_TOKEN`
  - `AWL_DOCKER_RELEASE_DOCKERHUB_IMAGE`, `AWL_DOCKER_RELEASE_GHCR_IMAGE`
  - `AWL_DOCKER_RELEASE_PUSH_LATEST`, `AWL_DOCKER_RELEASE_PUSH_SHA`, `AWL_DOCKER_RELEASE_PUSH_VERSION`
  - `AWL_DOCKER_RELEASE_PLATFORMS`
  - `AWL_DOCKER_RELEASE_AGENT_KIT_REF`

Outputs:

- Multi-arch image publish to Docker Hub + GHCR (linux/amd64, linux/arm64 by default).
- Pushed tags include `latest`, `sha-<short_sha>`, and optional `vX.Y.Z`.
- Clear failure output listing missing env/credentials or invalid conditions.

Exit codes:

- `0`: success
- `1`: failure
- `2`: usage error

Failure modes:

- Missing required credentials for enabled targets.
- Invalid `--version` format or version-tag policy mismatch.
- Missing/invalid `AGENT_KIT_REF`.
- Docker login/buildx/push failure.

## Scripts (only entrypoints)

- `<PROJECT_ROOT>/.agents/skills/release-docker-image/scripts/release-docker-image.sh`

## Workflow

1. Set publish env vars in `.env` (use `AWL_DOCKER_RELEASE_*`).
2. Run:
   - `.agents/skills/release-docker-image/scripts/release-docker-image.sh --version vX.Y.Z`
3. For preflight only:
   - `.agents/skills/release-docker-image/scripts/release-docker-image.sh --version vX.Y.Z --dry-run`
4. Verify pushed manifests:
   - `docker buildx imagetools inspect graysurf/agent-workspace-launcher:vX.Y.Z`
   - `docker buildx imagetools inspect ghcr.io/graysurf/agent-workspace-launcher:vX.Y.Z`
