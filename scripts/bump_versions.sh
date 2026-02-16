#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage:
  scripts/bump_versions.sh --from-main [--run-e2e]
  scripts/bump_versions.sh --agent-kit-ref <ref|sha> [--run-e2e]

options:
  --from-main                Resolve AGENT_KIT_REF from refs/heads/main.
  --agent-kit-ref <ref|sha>  Resolve AGENT_KIT_REF to a full 40-char commit SHA.

  --agent-kit-repo <url>     Override agent-kit repo URL (default: https://github.com/graysurf/agent-kit.git).
  --image-tag <tag>          Docker image tag to build (default: agent-workspace-launcher:local).

  --skip-checks              Skip ruff + pytest script_smoke + Rust checks.
  --skip-docker              Skip docker build and /opt/agent-kit/.ref verification.
  --run-e2e                  Run real-Docker e2e (pytest -m e2e) against the built image.

notes:
  - Always writes a resolved full commit SHA into VERSIONS.env.
  - Rust cutover removed zsh bundle generation and ZSH_KIT_REF pinning.
  - E2E forces AWL_E2E=1, AWL_E2E_FULL=1, and sets AWL_E2E_IMAGE=<image-tag>.
  - rm --all coverage is still gated by AWL_E2E_ALLOW_RM_ALL=1.
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

info() {
  echo "info: $*" >&2
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing command: $1"
}

is_full_sha() {
  local v="${1:-}"
  [[ "$v" =~ ^[0-9a-f]{40}$ ]]
}

resolve_ref() {
  local repo="$1"
  local ref="$2"

  if is_full_sha "$ref"; then
    echo "$ref"
    return 0
  fi

  local -a candidates=()
  if [[ "$ref" == refs/* ]]; then
    if [[ "$ref" == refs/tags/* && "$ref" != *'^{}' ]]; then
      candidates+=("${ref}^{}")
    fi
    candidates+=("$ref")
  else
    candidates+=("refs/heads/${ref}")
    candidates+=("refs/tags/${ref}^{}")
    candidates+=("refs/tags/${ref}")
    candidates+=("$ref")
  fi

  local line=''
  local sha=''
  local candidate=''
  for candidate in "${candidates[@]}"; do
    line="$(git ls-remote --exit-code "$repo" "$candidate" 2>/dev/null | head -n 1 || true)"
    sha="${line%%[[:space:]]*}"
    if is_full_sha "$sha"; then
      echo "$sha"
      return 0
    fi
  done

  die "cannot resolve ref: repo=$repo ref=$ref"
}

ensure_venv() {
  local repo_root="$1"

  require_cmd python3

  if [[ ! -x "${repo_root}/.venv/bin/python" ]]; then
    info "creating venv: .venv"
    python3 -m venv "${repo_root}/.venv"
  fi

  info "installing dev deps: requirements-dev.txt"
  "${repo_root}/.venv/bin/python" -m pip install -r "${repo_root}/requirements-dev.txt"
}

update_versions_file() {
  local versions_file="$1"
  local resolved_agent="$2"

  [[ -f "$versions_file" ]] || die "missing file: $versions_file"

  local tmp=''
  tmp="$(mktemp 2>/dev/null || true)"
  [[ -n "$tmp" ]] || tmp="/tmp/agent-workspace-launcher.versions.$$"

  local have_agent=0
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == ZSH_KIT_REF=* ]]; then
      continue
    fi
    if [[ "$line" == AGENT_KIT_REF=* ]]; then
      echo "AGENT_KIT_REF=${resolved_agent}" >>"$tmp"
      have_agent=1
      continue
    fi
    echo "$line" >>"$tmp"
  done <"$versions_file"

  if (( have_agent == 0 )); then
    echo "AGENT_KIT_REF=${resolved_agent}" >>"$tmp"
  fi

  mv -f -- "$tmp" "$versions_file"
}

main() {
  local from_main=0
  local agent_kit_ref=''

  local agent_kit_repo='https://github.com/graysurf/agent-kit.git'
  local image_tag='agent-workspace-launcher:local'

  local skip_checks=0
  local skip_docker=0
  local run_e2e=0

  while [[ $# -gt 0 ]]; do
    case "${1-}" in
      -h|--help)
        usage
        exit 0
        ;;
      --from-main)
        from_main=1
        shift
        ;;
      --agent-kit-ref)
        agent_kit_ref="${2-}"
        shift 2
        ;;
      --agent-kit-repo)
        agent_kit_repo="${2-}"
        shift 2
        ;;
      --image-tag)
        image_tag="${2-}"
        shift 2
        ;;
      --skip-checks)
        skip_checks=1
        shift
        ;;
      --skip-docker)
        skip_docker=1
        shift
        ;;
      --run-e2e)
        run_e2e=1
        shift
        ;;
      *)
        die "unknown argument: ${1-}"
        ;;
    esac
  done

  local repo_root=''
  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

  require_cmd git

  if ! git -C "$repo_root" diff --quiet --; then
    die "working tree is dirty (commit/stash first)"
  fi

  if (( from_main == 1 )); then
    agent_kit_ref='refs/heads/main'
  fi

  [[ -n "$agent_kit_ref" ]] || die "missing --agent-kit-ref (or use --from-main)"

  info "resolving AGENT_KIT_REF..."
  local resolved_agent=''
  resolved_agent="$(resolve_ref "$agent_kit_repo" "$agent_kit_ref")"
  info "resolved: AGENT_KIT_REF=$resolved_agent"

  info "updating: VERSIONS.env"
  update_versions_file "${repo_root}/VERSIONS.env" "$resolved_agent"

  ensure_venv "$repo_root"

  if (( skip_checks == 0 )); then
    info "running pre-submit checks..."
    (cd "$repo_root" && ./.venv/bin/python -m ruff format --check .)
    (cd "$repo_root" && ./.venv/bin/python -m ruff check .)
    (cd "$repo_root" && ./.venv/bin/python -m pytest -m script_smoke)
    (cd "$repo_root" && cargo fmt --all -- --check)
    (cd "$repo_root" && cargo check --workspace)
    (cd "$repo_root" && cargo test -p agent-workspace)
  else
    info "skipping checks (--skip-checks)"
  fi

  if (( skip_docker == 0 )); then
    require_cmd docker

    info "building image: ${image_tag}"
    (
      cd "$repo_root"
      docker build -t "$image_tag" \
        --build-arg AGENT_KIT_REF="$resolved_agent" \
        .
    )

    info "verifying AGENT_KIT_REF inside image..."
    local image_agent_ref=''
    image_agent_ref="$(docker run --rm --entrypoint cat "$image_tag" /opt/agent-kit/.ref | tr -d '\r\n')"

    if [[ "$image_agent_ref" != "$resolved_agent" ]]; then
      die "image AGENT_KIT_REF mismatch: expected=$resolved_agent got=$image_agent_ref"
    fi

    docker run --rm "$image_tag" --help >/dev/null
  else
    info "skipping docker build/verify (--skip-docker)"
  fi

  if (( run_e2e == 1 )); then
    require_cmd docker

    if (( skip_docker == 1 )); then
      if ! docker image inspect "$image_tag" >/dev/null 2>&1; then
        die "cannot run e2e: image not found (tag=${image_tag}); remove --skip-docker or build it first"
      fi
    fi

    local allow_rm_all="${AWL_E2E_ALLOW_RM_ALL:-}"
    local allow_rm_all_lc=''
    allow_rm_all_lc="$(printf '%s' "$allow_rm_all" | tr '[:upper:]' '[:lower:]')"
    if [[ "$allow_rm_all_lc" != "1" && "$allow_rm_all_lc" != "true" && "$allow_rm_all_lc" != "yes" && "$allow_rm_all_lc" != "on" ]]; then
      info "e2e note: rm --all coverage excluded unless AWL_E2E_ALLOW_RM_ALL=1"
    fi

    info "running real Docker e2e (full matrix) against: ${image_tag}"
    (
      cd "$repo_root"
      export AWL_E2E=1
      export AWL_E2E_FULL=1
      export AWL_E2E_IMAGE="$image_tag"
      unset AWL_E2E_CASE 2>/dev/null || true
      "${repo_root}/.venv/bin/python" -m pytest -m e2e -q -ra
    )
  fi

  info "done"
}

main "$@"
