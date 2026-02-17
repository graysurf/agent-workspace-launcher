#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  .agents/skills/release-homebrew/scripts/release-homebrew.sh \
    --version <vX.Y.Z> \
    [--date <YYYY-MM-DD>] \
    [--branch <name>] \
    [--skip-checks] \
    [--skip-smoke] \
    [--skip-changelog] \
    [--skip-audit]

Runs the Homebrew release preflight sequence:
  1) required repo checks from DEVELOPMENT.md
  2) local binary + awl alias smoke
  3) release_prepare_changelog.sh
  4) release_audit.sh --strict
USAGE
}

say() {
  printf 'release-homebrew: %s\n' "$*" >&2
}

die() {
  say "error: $*"
  exit 1
}

version=""
date_value=""
branch="main"
skip_checks=0
skip_smoke=0
skip_changelog=0
skip_audit=0

while [[ $# -gt 0 ]]; do
  case "${1:-}" in
    --version)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      version="${2:-}"
      shift 2
      ;;
    --date)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      date_value="${2:-}"
      shift 2
      ;;
    --branch)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      branch="${2:-}"
      shift 2
      ;;
    --skip-checks)
      skip_checks=1
      shift
      ;;
    --skip-smoke)
      skip_smoke=1
      shift
      ;;
    --skip-changelog)
      skip_changelog=1
      shift
      ;;
    --skip-audit)
      skip_audit=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "${version}" ]]; then
  usage >&2
  exit 2
fi

if [[ ! "${version}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z]+)*$ ]]; then
  say "error: invalid --version '${version}' (expected vX.Y.Z)"
  exit 2
fi

if [[ -n "${date_value}" && ! "${date_value}" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  say "error: invalid --date '${date_value}' (expected YYYY-MM-DD)"
  exit 2
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[[ -n "${repo_root}" ]] || die "must run inside a git work tree"
cd "${repo_root}"

if [[ -n "$(git status --porcelain)" ]]; then
  die "working tree must be clean before release preflight"
fi

run_required_checks() {
  local -a bash_scripts=()
  local -a zsh_scripts=()

  mapfile -t bash_scripts < <(git ls-files 'scripts/*.sh' 'scripts/*.bash')
  mapfile -t zsh_scripts < <(git ls-files 'scripts/*.zsh')

  if ((${#bash_scripts[@]} > 0)); then
    bash -n "${bash_scripts[@]}"
  fi
  if ((${#zsh_scripts[@]} > 0)); then
    zsh -n "${zsh_scripts[@]}"
  fi
  if ((${#bash_scripts[@]} > 0)); then
    shellcheck "${bash_scripts[@]}"
  fi

  .venv/bin/python -m ruff format --check .
  .venv/bin/python -m ruff check .
  .venv/bin/python -m pytest -m script_smoke

  cargo fmt --all -- --check
  cargo check --workspace
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test -p agent-workspace
}

run_binary_smoke() {
  local tmp_dir

  cargo build --release -p agent-workspace --bin agent-workspace-launcher
  ./target/release/agent-workspace-launcher --help >/dev/null

  tmp_dir="$(mktemp -d)"
  ln -sf "$(pwd)/target/release/agent-workspace-launcher" "${tmp_dir}/awl"
  "${tmp_dir}/awl" --help >/dev/null
}

if ((skip_checks == 0)); then
  say "running required checks from DEVELOPMENT.md"
  run_required_checks
fi

if ((skip_smoke == 0)); then
  say "running local binary smoke"
  run_binary_smoke
fi

if ((skip_changelog == 0)); then
  say "preparing changelog for ${version}"
  if [[ -n "${date_value}" ]]; then
    ./scripts/release_prepare_changelog.sh --version "${version}" --date "${date_value}"
  else
    ./scripts/release_prepare_changelog.sh --version "${version}"
  fi
fi

if ((skip_audit == 0)); then
  say "running strict release audit for ${version}"
  ./scripts/release_audit.sh --version "${version}" --branch "${branch}" --strict
fi

cat <<EOF
release-homebrew: preflight complete for ${version}
next:
  1) commit changelog via semantic-commit
  2) git -c tag.gpgSign=false tag ${version}
  3) git push origin ${version}
  4) verify release-brew workflow/assets
EOF
