#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  .agents/skills/release-docker-image/scripts/release-docker-image.sh \
    [--version <vX.Y.Z>] \
    [--ref <git-ref>] \
    [--platforms <list>] \
    [--dockerhub-image <name>] \
    [--ghcr-image <name>] \
    [--env-file <path>] \
    [--no-env-file] \
    [--publish-dockerhub|--no-publish-dockerhub] \
    [--publish-ghcr|--no-publish-ghcr] \
    [--push-latest|--no-push-latest] \
    [--push-sha|--no-push-sha] \
    [--push-version-tag|--no-push-version-tag] \
    [--dry-run]

Uses AWL_DOCKER_RELEASE_* env vars (optionally loaded from .env) and pushes
multi-arch images to Docker Hub + GHCR.
USAGE
}

say() {
  printf 'release-docker-image: %s\n' "$*" >&2
}

die() {
  say "error: $*"
  exit 1
}

to_bool() {
  local value
  value="$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')"
  case "${value}" in
    1|true|yes|on) echo 1 ;;
    0|false|no|off|'') echo 0 ;;
    *) return 1 ;;
  esac
}

to_push_version_policy() {
  local value
  value="$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')"
  case "${value}" in
    1|true|yes|on) echo 1 ;;
    0|false|no|off) echo 0 ;;
    auto|'') echo auto ;;
    *) return 1 ;;
  esac
}

assert_semver_tag() {
  local tag="${1:-}"
  [[ "${tag}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z]+)*$ ]]
}

args=("$@")
env_file=".env"
load_env_file=1
env_file_explicit=0

index=0
while ((index < ${#args[@]})); do
  case "${args[index]}" in
    --env-file)
      if ((index + 1 >= ${#args[@]})); then
        usage >&2
        exit 2
      fi
      env_file="${args[index + 1]}"
      env_file_explicit=1
      ((index += 2))
      ;;
    --no-env-file)
      load_env_file=0
      ((index += 1))
      ;;
    *)
      ((index += 1))
      ;;
  esac
done

if ((load_env_file == 1)); then
  if [[ -f "${env_file}" ]]; then
    if grep -Eq '^[[:space:]]*(export[[:space:]]+)?CWS_[A-Z0-9_]*=' "${env_file}"; then
      die "legacy CWS_* variables found in ${env_file}; rename to AWL_* before publishing"
    fi
    set -a
    # shellcheck disable=SC1090
    source "${env_file}"
    set +a
  elif ((env_file_explicit == 1)); then
    die "env file not found: ${env_file}"
  fi
fi

release_ref="${AWL_DOCKER_RELEASE_REF:-HEAD}"
release_version="${AWL_DOCKER_RELEASE_VERSION:-}"
platforms="${AWL_DOCKER_RELEASE_PLATFORMS:-linux/amd64,linux/arm64}"
dockerhub_image="${AWL_DOCKER_RELEASE_DOCKERHUB_IMAGE:-graysurf/agent-workspace-launcher}"
ghcr_image="${AWL_DOCKER_RELEASE_GHCR_IMAGE:-ghcr.io/graysurf/agent-workspace-launcher}"
publish_dockerhub_raw="${AWL_DOCKER_RELEASE_PUBLISH_DOCKERHUB:-1}"
publish_ghcr_raw="${AWL_DOCKER_RELEASE_PUBLISH_GHCR:-1}"
push_latest_raw="${AWL_DOCKER_RELEASE_PUSH_LATEST:-1}"
push_sha_raw="${AWL_DOCKER_RELEASE_PUSH_SHA:-1}"
push_version_raw="${AWL_DOCKER_RELEASE_PUSH_VERSION:-auto}"
dockerhub_username="${AWL_DOCKER_RELEASE_DOCKERHUB_USERNAME:-${DOCKERHUB_USERNAME:-}}"
dockerhub_token="${AWL_DOCKER_RELEASE_DOCKERHUB_TOKEN:-${DOCKERHUB_TOKEN:-}}"
ghcr_username="${AWL_DOCKER_RELEASE_GHCR_USERNAME:-${GHCR_USERNAME:-${GITHUB_ACTOR:-}}}"
ghcr_token="${AWL_DOCKER_RELEASE_GHCR_TOKEN:-${GHCR_TOKEN:-${CR_PAT:-${GITHUB_TOKEN:-}}}}"
agent_kit_ref="${AWL_DOCKER_RELEASE_AGENT_KIT_REF:-}"
dry_run=0

while [[ $# -gt 0 ]]; do
  case "${1:-}" in
    --version)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      release_version="${2:-}"
      shift 2
      ;;
    --ref)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      release_ref="${2:-}"
      shift 2
      ;;
    --platforms)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      platforms="${2:-}"
      shift 2
      ;;
    --dockerhub-image)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      dockerhub_image="${2:-}"
      shift 2
      ;;
    --ghcr-image)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      ghcr_image="${2:-}"
      shift 2
      ;;
    --dockerhub-username)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      dockerhub_username="${2:-}"
      shift 2
      ;;
    --dockerhub-token)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      dockerhub_token="${2:-}"
      shift 2
      ;;
    --ghcr-username)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      ghcr_username="${2:-}"
      shift 2
      ;;
    --ghcr-token)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      ghcr_token="${2:-}"
      shift 2
      ;;
    --agent-kit-ref)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      agent_kit_ref="${2:-}"
      shift 2
      ;;
    --publish-dockerhub)
      publish_dockerhub_raw=1
      shift
      ;;
    --no-publish-dockerhub)
      publish_dockerhub_raw=0
      shift
      ;;
    --publish-ghcr)
      publish_ghcr_raw=1
      shift
      ;;
    --no-publish-ghcr)
      publish_ghcr_raw=0
      shift
      ;;
    --push-latest)
      push_latest_raw=1
      shift
      ;;
    --no-push-latest)
      push_latest_raw=0
      shift
      ;;
    --push-sha)
      push_sha_raw=1
      shift
      ;;
    --no-push-sha)
      push_sha_raw=0
      shift
      ;;
    --push-version-tag)
      push_version_raw=1
      shift
      ;;
    --no-push-version-tag)
      push_version_raw=0
      shift
      ;;
    --env-file)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      shift 2
      ;;
    --no-env-file)
      shift
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: ${1:-}" >&2
      usage >&2
      exit 2
      ;;
  esac
done

publish_dockerhub="$(to_bool "${publish_dockerhub_raw}" || true)"
[[ -n "${publish_dockerhub}" ]] || {
  usage >&2
  exit 2
}
publish_ghcr="$(to_bool "${publish_ghcr_raw}" || true)"
[[ -n "${publish_ghcr}" ]] || {
  usage >&2
  exit 2
}
push_latest="$(to_bool "${push_latest_raw}" || true)"
[[ -n "${push_latest}" ]] || {
  usage >&2
  exit 2
}
push_sha="$(to_bool "${push_sha_raw}" || true)"
[[ -n "${push_sha}" ]] || {
  usage >&2
  exit 2
}
push_version_policy="$(to_push_version_policy "${push_version_raw}" || true)"
[[ -n "${push_version_policy}" ]] || {
  usage >&2
  exit 2
}

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[[ -n "${repo_root}" ]] || die "must run inside a git work tree"
cd "${repo_root}"

commit_sha="$(git rev-parse --verify "${release_ref}^{commit}" 2>/dev/null || true)"
[[ -n "${commit_sha}" ]] || die "unable to resolve --ref '${release_ref}'"

short_sha="$(git rev-parse --short=7 "${commit_sha}" 2>/dev/null || true)"
if [[ ! "${short_sha}" =~ ^[0-9a-f]{7}$ ]]; then
  die "unable to derive 7-char short sha for ref '${release_ref}'"
fi

if [[ -z "${release_version}" ]]; then
  maybe_tag="$(git describe --tags --exact-match "${commit_sha}" 2>/dev/null || true)"
  if assert_semver_tag "${maybe_tag}"; then
    release_version="${maybe_tag}"
  fi
fi

if [[ -n "${release_version}" ]] && ! assert_semver_tag "${release_version}"; then
  say "error: invalid --version '${release_version}' (expected vX.Y.Z)"
  exit 2
fi

push_version=0
if [[ "${push_version_policy}" == "auto" ]]; then
  if [[ -n "${release_version}" ]]; then
    push_version=1
  fi
elif [[ "${push_version_policy}" == "1" ]]; then
  push_version=1
fi

if ((push_version == 1)) && [[ -z "${release_version}" ]]; then
  die "push version tag is enabled but no release version was provided/resolved"
fi

if [[ -z "${agent_kit_ref}" ]]; then
  if [[ ! -f VERSIONS.env ]]; then
    die "missing VERSIONS.env (or set AWL_DOCKER_RELEASE_AGENT_KIT_REF)"
  fi
  if grep -Eq '^[[:space:]]*ZSH_KIT_REF=' VERSIONS.env; then
    die "legacy ZSH_KIT_REF found in VERSIONS.env"
  fi
  # shellcheck disable=SC1091
  source VERSIONS.env
  agent_kit_ref="${AGENT_KIT_REF:-}"
fi

if [[ ! "${agent_kit_ref}" =~ ^[0-9a-f]{40}$ ]]; then
  die "AGENT_KIT_REF must be a full 40-char sha (got '${agent_kit_ref}')"
fi

if ((publish_dockerhub == 0 && publish_ghcr == 0)); then
  die "both publish targets are disabled"
fi

missing=()
if ((publish_dockerhub == 1)); then
  [[ -n "${dockerhub_image}" ]] || missing+=("AWL_DOCKER_RELEASE_DOCKERHUB_IMAGE")
  [[ -n "${dockerhub_username}" ]] || missing+=("AWL_DOCKER_RELEASE_DOCKERHUB_USERNAME")
  [[ -n "${dockerhub_token}" ]] || missing+=("AWL_DOCKER_RELEASE_DOCKERHUB_TOKEN")
fi
if ((publish_ghcr == 1)); then
  [[ -n "${ghcr_image}" ]] || missing+=("AWL_DOCKER_RELEASE_GHCR_IMAGE")
  [[ -n "${ghcr_username}" ]] || missing+=("AWL_DOCKER_RELEASE_GHCR_USERNAME")
  [[ -n "${ghcr_token}" ]] || missing+=("AWL_DOCKER_RELEASE_GHCR_TOKEN")
fi

if ((${#missing[@]} > 0)); then
  say "missing required publish parameters:"
  for item in "${missing[@]}"; do
    say "  - ${item}"
  done
  exit 1
fi

tags=()
if ((publish_dockerhub == 1)); then
  if ((push_latest == 1)); then
    tags+=("${dockerhub_image}:latest")
  fi
  if ((push_sha == 1)); then
    tags+=("${dockerhub_image}:sha-${short_sha}")
  fi
  if ((push_version == 1)); then
    tags+=("${dockerhub_image}:${release_version}")
  fi
fi
if ((publish_ghcr == 1)); then
  if ((push_latest == 1)); then
    tags+=("${ghcr_image}:latest")
  fi
  if ((push_sha == 1)); then
    tags+=("${ghcr_image}:sha-${short_sha}")
  fi
  if ((push_version == 1)); then
    tags+=("${ghcr_image}:${release_version}")
  fi
fi

if ((${#tags[@]} == 0)); then
  die "no tags selected; enable at least one of latest/sha/version outputs"
fi

if ((dry_run == 1)); then
  say "dry-run only (no docker login/build/push)"
else
  command -v docker >/dev/null 2>&1 || die "docker is required"
  docker buildx version >/dev/null 2>&1 || die "docker buildx is required"

  if ((publish_dockerhub == 1)); then
    say "logging in to Docker Hub as ${dockerhub_username}"
    printf '%s' "${dockerhub_token}" | docker login --username "${dockerhub_username}" --password-stdin >/dev/null
  fi

  if ((publish_ghcr == 1)); then
    say "logging in to GHCR as ${ghcr_username}"
    printf '%s' "${ghcr_token}" | docker login ghcr.io --username "${ghcr_username}" --password-stdin >/dev/null
  fi
fi

build_cmd=(docker buildx build --platform "${platforms}" --push --build-arg "AGENT_KIT_REF=${agent_kit_ref}")
for tag in "${tags[@]}"; do
  build_cmd+=(--tag "${tag}")
done
build_cmd+=(".")

say "ref: ${release_ref} (${commit_sha})"
if [[ -n "${release_version}" ]]; then
  say "version: ${release_version}"
else
  say "version: <none> (version tag disabled)"
fi
say "platforms: ${platforms}"
say "tags:"
for tag in "${tags[@]}"; do
  say "  - ${tag}"
done

if ((dry_run == 1)); then
  say "build command:"
  printf '%q ' "${build_cmd[@]}" >&2
  printf '\n' >&2
  exit 0
fi

"${build_cmd[@]}"

say "publish complete"
