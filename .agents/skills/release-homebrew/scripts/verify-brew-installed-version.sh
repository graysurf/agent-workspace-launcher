#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  .agents/skills/release-homebrew/scripts/verify-brew-installed-version.sh \
    --version <vX.Y.Z> \
    [--formula <name>] \
    [--tap <owner/name>] \
    [--tap-repo <path>]

What it does:
  1) Optional: point brew tap to a local tap repo path
  2) brew update-reset on the target tap
  3) brew upgrade/install the formula from that tap
  4) verify installed version matches --version
  5) verify both PATH commands report expected version
  6) verify bash/zsh completion files are present
USAGE
}

say() {
  printf 'verify-brew-version: %s\n' "$*" >&2
}

die() {
  say "error: $*"
  exit 1
}

extract_semver() {
  local text="${1:-}"
  printf '%s\n' "${text}" | grep -Eo '[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z]+)*' | head -n 1
}

version=""
tap_name="graysurf/tap"
tap_repo=""
formula_name="agent-workspace-launcher"

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
    --tap)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      tap_name="${2:-}"
      shift 2
      ;;
    --formula)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      formula_name="${2:-}"
      shift 2
      ;;
    --tap-repo)
      [[ $# -ge 2 ]] || {
        usage >&2
        exit 2
      }
      tap_repo="${2:-}"
      shift 2
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

[[ -n "${version}" ]] || {
  usage >&2
  exit 2
}

if [[ ! "${version}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z]+)*$ ]]; then
  die "invalid --version '${version}' (expected vX.Y.Z)"
fi

expected_version="${version#v}"
tap_formula="${tap_name}/${formula_name}"

command -v brew >/dev/null 2>&1 || die "brew is required"

if [[ -n "${tap_repo}" ]]; then
  [[ -d "${tap_repo}" ]] || die "tap repo not found: ${tap_repo}"
  say "pointing tap ${tap_name} to ${tap_repo}"
  brew tap "${tap_name}" "${tap_repo}" --custom-remote >/dev/null
elif ! brew tap | grep -Fx "${tap_name}" >/dev/null; then
  say "tapping ${tap_name}"
  brew tap "${tap_name}" >/dev/null
fi

tap_repo_path="$(brew --repo "${tap_name}" 2>/dev/null || true)"
[[ -n "${tap_repo_path}" ]] || die "tap not found: ${tap_name}"

say "updating tap checkout: ${tap_repo_path}"
brew update-reset "${tap_repo_path}" >/dev/null

if HOMEBREW_NO_AUTO_UPDATE=1 brew list --versions "${formula_name}" >/dev/null 2>&1; then
  say "upgrading formula ${tap_formula}"
  if ! HOMEBREW_NO_AUTO_UPDATE=1 brew upgrade "${tap_formula}" >/dev/null 2>&1; then
    say "upgrade returned non-zero; retrying with reinstall"
    HOMEBREW_NO_AUTO_UPDATE=1 brew reinstall "${tap_formula}" >/dev/null
  fi
else
  say "installing formula ${tap_formula}"
  HOMEBREW_NO_AUTO_UPDATE=1 brew install "${tap_formula}" >/dev/null
fi

installed_version="$(HOMEBREW_NO_AUTO_UPDATE=1 brew list --versions "${formula_name}" | awk '{print $2}')"
[[ -n "${installed_version}" ]] || die "unable to read installed brew version"

if [[ "${installed_version}" != "${expected_version}" ]]; then
  die "installed formula version is ${installed_version}, expected ${expected_version}"
fi

formula_prefix="$(brew --prefix "${formula_name}" 2>/dev/null || true)"
[[ -n "${formula_prefix}" ]] || die "unable to resolve brew prefix for formula"

launcher_bin="${formula_prefix}/bin/agent-workspace-launcher"
awl_bin="${formula_prefix}/bin/awl"
[[ -x "${launcher_bin}" ]] || die "missing executable: ${launcher_bin}"
[[ -x "${awl_bin}" ]] || die "missing executable: ${awl_bin}"

launcher_out="$("${launcher_bin}" --version 2>/dev/null || true)"
awl_out="$("${awl_bin}" --version 2>/dev/null || true)"
launcher_ver="$(extract_semver "${launcher_out}")"
awl_ver="$(extract_semver "${awl_out}")"

[[ -n "${launcher_ver}" ]] || die "unable to parse launcher version from: ${launcher_out}"
[[ -n "${awl_ver}" ]] || die "unable to parse awl version from: ${awl_out}"

if [[ "${launcher_ver}" != "${expected_version}" ]]; then
  die "agent-workspace-launcher reports ${launcher_ver}, expected ${expected_version}"
fi
if [[ "${awl_ver}" != "${expected_version}" ]]; then
  die "awl reports ${awl_ver}, expected ${expected_version}"
fi

verify_path_command() {
  local cmd="$1"
  local cmd_path
  local cmd_out
  local cmd_ver

  cmd_path="$(command -v "${cmd}" 2>/dev/null || true)"
  [[ -n "${cmd_path}" ]] || die "command not found on PATH: ${cmd}"

  cmd_out="$("${cmd}" --version 2>/dev/null || true)"
  cmd_ver="$(extract_semver "${cmd_out}")"
  [[ -n "${cmd_ver}" ]] || die "unable to parse ${cmd} version from: ${cmd_out}"

  if [[ "${cmd_ver}" != "${expected_version}" ]]; then
    die "${cmd} on PATH (${cmd_path}) reports ${cmd_ver}, expected ${expected_version}"
  fi
}

verify_path_command "agent-workspace-launcher"
verify_path_command "awl"

brew_prefix="$(brew --prefix)"
bash_comp="${brew_prefix}/etc/bash_completion.d/agent-workspace-launcher"
zsh_comp="${brew_prefix}/share/zsh/site-functions/_agent-workspace-launcher"

[[ -f "${bash_comp}" ]] || die "missing bash completion file: ${bash_comp}"
[[ -f "${zsh_comp}" ]] || die "missing zsh completion file: ${zsh_comp}"

cat <<EOF
verify-brew-version: ok
  tap: ${tap_name}
  formula: ${formula_name}
  expected: ${expected_version}
  installed: ${installed_version}
  launcher: ${launcher_bin}
  awl: ${awl_bin}
  launcher(path): $(command -v agent-workspace-launcher)
  awl(path): $(command -v awl)
  bash completion: ${bash_comp}
  zsh completion: ${zsh_comp}
EOF
