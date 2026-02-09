#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
usage:
  scripts/bump_versions.sh --from-main [--run-e2e]
  scripts/bump_versions.sh --zsh-kit-ref <ref|sha> --codex-kit-ref <ref|sha> [--run-e2e]

options:
  --from-main              Resolve upstream pins from default release branches:
                           zsh-kit=refs/heads/nils-cli, codex-kit=refs/heads/main.
  --zsh-kit-ref <ref|sha>  Resolve zsh-kit ref to a full 40-char commit SHA.
  --codex-kit-ref <ref|sha> Resolve codex-kit ref to a full 40-char commit SHA.

  --zsh-kit-repo <url>     Override zsh-kit repo URL (default: https://github.com/graysurf/zsh-kit.git).
  --codex-kit-repo <url>   Override codex-kit repo URL (default: https://github.com/graysurf/codex-kit.git).
  --image-tag <tag>        Docker image tag to build (default: codex-workspace-launcher:local).

  --skip-checks            Skip ruff + pytest script_smoke.
  --skip-docker            Skip docker build and /opt/*.ref verification.
  --skip-bundle            Skip regenerating bin/codex-workspace.
  --run-e2e                Run real-Docker e2e (pytest -m e2e) against the built image.

notes:
  - Always writes resolved commit SHAs into VERSIONS.env (no "main" drift).
  - Bundling uses the pinned zsh-kit tool (tools/bundle-wrapper.zsh) when available.
  - E2E forces CWS_E2E=1, CWS_E2E_FULL=1, and sets CWS_E2E_IMAGE=<image-tag>.
  - rm --all coverage is still gated by CWS_E2E_ALLOW_RM_ALL=1.
EOF
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

main() {
  local from_main=0
  local zsh_kit_ref=''
  local codex_kit_ref=''

  local zsh_kit_repo='https://github.com/graysurf/zsh-kit.git'
  local codex_kit_repo='https://github.com/graysurf/codex-kit.git'
  local image_tag='codex-workspace-launcher:local'

  local skip_checks=0
  local skip_docker=0
  local skip_bundle=0
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
      --zsh-kit-ref)
        zsh_kit_ref="${2-}"
        shift 2
        ;;
      --codex-kit-ref)
        codex_kit_ref="${2-}"
        shift 2
        ;;
      --zsh-kit-repo)
        zsh_kit_repo="${2-}"
        shift 2
        ;;
      --codex-kit-repo)
        codex_kit_repo="${2-}"
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
      --skip-bundle)
        skip_bundle=1
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
  require_cmd zsh

  if ! git -C "$repo_root" diff --quiet --; then
    die "working tree is dirty (commit/stash first)"
  fi

  if (( from_main == 1 )); then
    zsh_kit_ref="refs/heads/nils-cli"
    codex_kit_ref="refs/heads/main"
  fi

  [[ -n "$zsh_kit_ref" ]] || die "missing --zsh-kit-ref (or use --from-main)"
  [[ -n "$codex_kit_ref" ]] || die "missing --codex-kit-ref (or use --from-main)"

  info "resolving refs..."
  local resolved_zsh=''
  local resolved_codex=''
  resolved_zsh="$(resolve_ref "$zsh_kit_repo" "$zsh_kit_ref")"
  resolved_codex="$(resolve_ref "$codex_kit_repo" "$codex_kit_ref")"

  info "resolved: ZSH_KIT_REF=$resolved_zsh"
  info "resolved: CODEX_KIT_REF=$resolved_codex"

  local versions_file="${repo_root}/VERSIONS.env"
  [[ -f "$versions_file" ]] || die "missing file: VERSIONS.env"

  info "updating: VERSIONS.env"
  local tmp=''
  tmp="$(mktemp 2>/dev/null || true)"
  [[ -n "$tmp" ]] || tmp="/tmp/codex-workspace-launcher.versions.$$"

  local have_zsh=0
  local have_codex=0
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == ZSH_KIT_REF=* ]]; then
      echo "ZSH_KIT_REF=${resolved_zsh}" >>"$tmp"
      have_zsh=1
      continue
    fi
    if [[ "$line" == CODEX_KIT_REF=* ]]; then
      echo "CODEX_KIT_REF=${resolved_codex}" >>"$tmp"
      have_codex=1
      continue
    fi
    echo "$line" >>"$tmp"
  done <"$versions_file"

  if (( have_zsh == 0 )); then
    echo "ZSH_KIT_REF=${resolved_zsh}" >>"$tmp"
  fi
  if (( have_codex == 0 )); then
    echo "CODEX_KIT_REF=${resolved_codex}" >>"$tmp"
  fi

  mv -f -- "$tmp" "$versions_file"

  if (( skip_bundle == 0 )); then
    info "regenerating bundle: bin/codex-workspace"
    (cd "$repo_root" && ./scripts/generate_codex_workspace_bundle.sh)
  else
    info "skipping bundle regeneration (--skip-bundle)"
  fi

  ensure_venv "$repo_root"

  if (( skip_checks == 0 )); then
    info "running pre-submit checks..."
    (cd "$repo_root" && ./.venv/bin/python -m ruff format --check .)
    (cd "$repo_root" && ./.venv/bin/python -m ruff check .)
    (cd "$repo_root" && ./.venv/bin/python -m pytest -m script_smoke)
  else
    info "skipping pre-submit checks (--skip-checks)"
  fi

  if (( skip_docker == 0 )); then
    require_cmd docker

    info "building image: ${image_tag}"
    (
      cd "$repo_root"
      docker build -t "$image_tag" \
        --build-arg ZSH_KIT_REF="$resolved_zsh" \
        --build-arg CODEX_KIT_REF="$resolved_codex" \
        .
    )

    info "verifying pins inside image..."
    local image_zsh_ref=''
    local image_codex_ref=''
    image_zsh_ref="$(docker run --rm --entrypoint cat "$image_tag" /opt/zsh-kit.ref | tr -d '\r\n')"
    image_codex_ref="$(docker run --rm --entrypoint cat "$image_tag" /opt/codex-kit/.ref | tr -d '\r\n')"

    if [[ "$image_zsh_ref" != "$resolved_zsh" ]]; then
      die "image zsh-kit ref mismatch: expected=$resolved_zsh got=$image_zsh_ref"
    fi
    if [[ "$image_codex_ref" != "$resolved_codex" ]]; then
      die "image codex-kit ref mismatch: expected=$resolved_codex got=$image_codex_ref"
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

    local allow_rm_all="${CWS_E2E_ALLOW_RM_ALL:-}"
    local allow_rm_all_lc=''
    allow_rm_all_lc="$(printf '%s' "$allow_rm_all" | tr '[:upper:]' '[:lower:]')"
    if [[ "$allow_rm_all_lc" != "1" && "$allow_rm_all_lc" != "true" && "$allow_rm_all_lc" != "yes" && "$allow_rm_all_lc" != "on" ]]; then
      info "e2e note: rm --all coverage excluded unless CWS_E2E_ALLOW_RM_ALL=1"
    fi

    info "running real Docker e2e (full matrix) against: ${image_tag}"
    (
      cd "$repo_root"
      export CWS_E2E=1
      export CWS_E2E_FULL=1
      export CWS_E2E_IMAGE="$image_tag"
      unset CWS_E2E_CASE 2>/dev/null || true
      "${repo_root}/.venv/bin/python" -m pytest -m e2e -q -ra
    )
  fi

  info "done"
  info "next:"
  info "  git diff"
  info "  git add -A && git commit"
  info "  gh pr create"
}

main "$@"
