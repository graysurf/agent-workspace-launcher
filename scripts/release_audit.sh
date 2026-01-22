#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/release_audit.sh --version <vX.Y.Z> [--branch <name>] [--strict]

checks:
  - Working tree is clean
  - Optional: current branch matches --branch
  - Tag does not already exist locally
  - CHANGELOG.md contains: ## vX.Y.Z - YYYY-MM-DD
  - Release entry records pinned upstream refs from VERSIONS.env:
      ### Upstream pins
      - zsh-kit: <ZSH_KIT_REF>
      - codex-kit: <CODEX_KIT_REF>
  - Release entry contains no placeholder lines (e.g. `- None`, `- ...`, `...`, `vX.Y.Z`, `YYYY-MM-DD`)

exit:
  - 0: all checks pass
  - 1: at least one check failed
  - 2: usage error
EOF
}

die() {
  echo "release_audit: $*" >&2
  exit 2
}

say_ok() { printf "ok: %s\n" "$1"; }
say_fail() { printf "fail: %s\n" "$1" >&2; }
say_warn() { printf "warn: %s\n" "$1" >&2; }

is_full_sha() {
  local v="${1:-}"
  [[ "$v" =~ ^[0-9a-f]{40}$ ]]
}

read_pins() {
  local file="$1"
  local zsh=''
  local codex=''
  while IFS= read -r line || [[ -n "$line" ]]; do
    case "$line" in
      \#*|'') continue ;;
      ZSH_KIT_REF=*)
        zsh="${line#ZSH_KIT_REF=}"
        zsh="${zsh%$'\r'}"
        zsh="${zsh%\"}"
        zsh="${zsh#\"}"
        zsh="${zsh%\'}"
        zsh="${zsh#\'}"
        ;;
      CODEX_KIT_REF=*)
        codex="${line#CODEX_KIT_REF=}"
        codex="${codex%$'\r'}"
        codex="${codex%\"}"
        codex="${codex#\"}"
        codex="${codex%\'}"
        codex="${codex#\'}"
        ;;
    esac
  done <"$file"

  [[ -n "$zsh" ]] || die "missing ZSH_KIT_REF in $file"
  [[ -n "$codex" ]] || die "missing CODEX_KIT_REF in $file"

  printf '%s %s\n' "$zsh" "$codex"
}

extract_release_notes() {
  local changelog="$1"
  local version="$2"
  awk -v v="$version" '
    $0 ~ "^## " v " " { f=1; heading=NR }
    f {
      if (NR > heading && $0 ~ "^## ") { exit }
      print
    }
  ' "$changelog" 2>/dev/null || true
}

has_placeholder_lines() {
  local text="$1"
  if printf '%s\n' "$text" | grep -qE '^[[:space:]]*-[[:space:]]+None[[:space:]]*$'; then
    return 0
  fi
  if printf '%s\n' "$text" | grep -qE '^[[:space:]]*-[[:space:]]+\\.{3}[[:space:]]*$'; then
    return 0
  fi
  if printf '%s\n' "$text" | grep -qE '^[[:space:]]*\\.{3}[[:space:]]*$'; then
    return 0
  fi
  if printf '%s\n' "$text" | grep -q 'vX.Y.Z'; then
    return 0
  fi
  if printf '%s\n' "$text" | grep -q 'YYYY-MM-DD'; then
    return 0
  fi
  if printf '%s\n' "$text" | grep -q '<!--'; then
    return 0
  fi
  return 1
}

main() {
  local version=''
  local branch=''
  local strict=0

  while [[ $# -gt 0 ]]; do
    case "${1-}" in
      -h|--help)
        usage
        exit 0
        ;;
      --version)
        version="${2-}"
        shift 2
        ;;
      --branch)
        branch="${2-}"
        shift 2
        ;;
      --strict)
        strict=1
        shift
        ;;
      *)
        die "unknown argument: ${1-}"
        ;;
    esac
  done

  [[ -n "$version" ]] || die "missing --version (expected vX.Y.Z)"

  local failed=0

  command -v git >/dev/null 2>&1 || die "git is required"
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    die "not in a git repository"
  fi

  if [[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    say_ok "version format: $version"
  else
    if (( strict )); then
      say_fail "version format invalid (expected vX.Y.Z): $version"
      failed=1
    else
      say_warn "version format unusual (expected vX.Y.Z): $version"
    fi
  fi

  if [[ -n "$(git status --porcelain 2>/dev/null || true)" ]]; then
    say_fail "working tree not clean (commit/stash changes first)"
    failed=1
  else
    say_ok "working tree clean"
  fi

  local current_branch=''
  current_branch="$(git branch --show-current 2>/dev/null || true)"
  if [[ -n "$branch" ]]; then
    if [[ "$current_branch" != "$branch" ]]; then
      say_fail "branch mismatch (current=$current_branch expected=$branch)"
      failed=1
    else
      say_ok "on branch $branch"
    fi
  fi

  if git show-ref --tags --verify --quiet "refs/tags/$version" 2>/dev/null; then
    say_fail "tag already exists: $version"
    failed=1
  else
    say_ok "tag not present: $version"
  fi

  local changelog='CHANGELOG.md'
  local versions_file='VERSIONS.env'

  if [[ ! -f "$changelog" ]]; then
    say_fail "missing changelog: $changelog"
    failed=1
  else
    say_ok "changelog present: $changelog"
  fi

  if [[ ! -f "$versions_file" ]]; then
    say_fail "missing versions file: $versions_file"
    failed=1
  else
    say_ok "versions file present: $versions_file"
  fi

  local zsh_ref=''
  local codex_ref=''
  if [[ -f "$versions_file" ]]; then
    read -r zsh_ref codex_ref < <(read_pins "$versions_file")
    say_ok "pins loaded: zsh-kit=${zsh_ref} codex-kit=${codex_ref}"
    if ! is_full_sha "$zsh_ref"; then
      if (( strict )); then
        say_fail "ZSH_KIT_REF is not a full 40-char sha: $zsh_ref"
        failed=1
      else
        say_warn "ZSH_KIT_REF is not a full 40-char sha: $zsh_ref"
      fi
    fi
    if ! is_full_sha "$codex_ref"; then
      if (( strict )); then
        say_fail "CODEX_KIT_REF is not a full 40-char sha: $codex_ref"
        failed=1
      else
        say_warn "CODEX_KIT_REF is not a full 40-char sha: $codex_ref"
      fi
    fi
  fi

  if [[ -f "$changelog" ]]; then
    if ! grep -qF "## ${version} - " "$changelog"; then
      say_fail "missing changelog heading: ## ${version} - YYYY-MM-DD"
      failed=1
    else
      say_ok "changelog entry exists: $version"
    fi

    local notes=''
    notes="$(extract_release_notes "$changelog" "$version")"
    if [[ -z "$notes" ]]; then
      say_fail "unable to extract notes for $version from $changelog"
      failed=1
    else
      if [[ "$notes" != *$'\n'"### Upstream pins"$'\n'* ]]; then
        say_fail "missing section: ### Upstream pins"
        failed=1
      else
        say_ok "section present: ### Upstream pins"
      fi

      if [[ -n "$zsh_ref" && "$notes" != *"- zsh-kit: ${zsh_ref}"* ]]; then
        say_fail "missing or mismatched pin line: - zsh-kit: ${zsh_ref}"
        failed=1
      else
        [[ -n "$zsh_ref" ]] && say_ok "pin recorded: zsh-kit"
      fi

      if [[ -n "$codex_ref" && "$notes" != *"- codex-kit: ${codex_ref}"* ]]; then
        say_fail "missing or mismatched pin line: - codex-kit: ${codex_ref}"
        failed=1
      else
        [[ -n "$codex_ref" ]] && say_ok "pin recorded: codex-kit"
      fi

      if has_placeholder_lines "$notes"; then
        if (( strict )); then
          say_fail "placeholder content detected in changelog entry for ${version} (remove placeholders before release)"
          failed=1
        else
          say_warn "placeholder content detected in changelog entry for ${version} (remove placeholders before release)"
        fi
      else
        say_ok "no placeholders detected in changelog entry: $version"
      fi
    fi
  fi

  if command -v gh >/dev/null 2>&1; then
    if gh auth status >/dev/null 2>&1; then
      say_ok "gh auth status"
    else
      if (( strict )); then
        say_fail "gh auth status failed (run: gh auth login)"
        failed=1
      else
        say_warn "gh auth status failed (run: gh auth login)"
      fi
    fi
  else
    say_warn "gh not installed; skipping gh auth check"
  fi

  if (( failed )); then
    exit 1
  fi
  exit 0
}

main "$@"
