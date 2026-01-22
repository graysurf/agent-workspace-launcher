#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
usage:
  scripts/generate_codex_workspace_bundle.sh

notes:
  - Regenerates ./bin/codex-workspace from the pinned ZSH_KIT_REF in VERSIONS.env.
  - Requires: git and zsh (bundling uses zsh-kit's tools/bundle-wrapper.zsh at the pinned ref).
EOF
}

if [[ "${1-}" == "-h" || "${1-}" == "--help" ]]; then
  usage
  exit 0
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
versions_file="${repo_root}/VERSIONS.env"

if [[ ! -f "$versions_file" ]]; then
  echo "error: missing VERSIONS.env: $versions_file" >&2
  exit 1
fi

# shellcheck disable=SC1090
source "$versions_file"

if [[ -z "${ZSH_KIT_REF:-}" ]]; then
  echo "error: VERSIONS.env must set ZSH_KIT_REF" >&2
  exit 1
fi

if ! command -v zsh >/dev/null 2>&1; then
  echo "error: zsh not found on PATH" >&2
  exit 1
fi

if ! command -v git >/dev/null 2>&1; then
  echo "error: git not found on PATH" >&2
  exit 1
fi

tmpdir="$(mktemp -d 2>/dev/null || true)"
if [[ -z "$tmpdir" ]]; then
  tmpdir="/tmp/codex-workspace-launcher.bundle.$$"
  mkdir -p -- "$tmpdir"
fi
cleanup() { rm -rf -- "$tmpdir" >/dev/null 2>&1 || true; }
trap cleanup EXIT

zsh_kit_repo="${ZSH_KIT_REPO:-https://github.com/graysurf/zsh-kit.git}"
zsh_kit_dir="${tmpdir}/zsh-kit"
git init -b main "$zsh_kit_dir" >/dev/null
git -C "$zsh_kit_dir" remote add origin "$zsh_kit_repo"
git -C "$zsh_kit_dir" fetch --depth 1 origin "$ZSH_KIT_REF" >/dev/null
git -C "$zsh_kit_dir" checkout --detach FETCH_HEAD >/dev/null
resolved_zsh_kit_ref="$(git -C "$zsh_kit_dir" rev-parse HEAD)"

bundle_wrapper="${zsh_kit_dir}/tools/bundle-wrapper.zsh"
if [[ ! -f "$bundle_wrapper" ]]; then
  bundle_wrapper="${HOME}/.config/zsh/tools/bundle-wrapper.zsh"
fi
if [[ ! -f "$bundle_wrapper" ]]; then
  echo "error: bundle-wrapper.zsh not found (expected in pinned zsh-kit or locally)" >&2
  echo "hint: check ZSH_KIT_REF=$ZSH_KIT_REF or install zsh-kit locally (~/.config/zsh)" >&2
  exit 1
fi

manifest="${repo_root}/scripts/bundles/codex-workspace.wrapper.zsh"
if [[ ! -f "$manifest" ]]; then
  echo "error: missing bundle manifest: $manifest" >&2
  exit 1
fi

for required in \
  "scripts/_features/codex-workspace/alias.zsh" \
  "scripts/_features/codex-workspace/repo-reset.zsh" \
  "scripts/_features/codex-workspace/workspace-rm.zsh" \
  "scripts/_features/codex-workspace/workspace-rsync.zsh" \
  "scripts/_features/codex-workspace/workspace-launcher.zsh"
do
  if [[ ! -f "${zsh_kit_dir}/${required}" ]]; then
    echo "error: missing expected zsh-kit file: ${required}" >&2
    echo "hint: check ZSH_KIT_REF=$ZSH_KIT_REF and ensure it contains codex-workspace" >&2
    exit 1
  fi
done

output="${repo_root}/bin/codex-workspace"
tmp_output="${tmpdir}/codex-workspace.bundled"

ZDOTDIR="${zsh_kit_dir}" \
ZSH_CONFIG_DIR="${zsh_kit_dir}/config" \
ZSH_BOOTSTRAP_SCRIPT_DIR="${zsh_kit_dir}/bootstrap" \
ZSH_SCRIPT_DIR="${zsh_kit_dir}/scripts" \
  zsh -f "$bundle_wrapper" \
  --input "$manifest" \
  --output "$tmp_output" \
  --entry codex-workspace

normalized_bundled_from="scripts/bundles/codex-workspace.wrapper.zsh"
tmp_output2="${tmpdir}/codex-workspace.bundled.header"

{
  IFS= read -r first || true
  if [[ -z "${first}" ]]; then
    echo "error: bundle output is empty" >&2
    exit 1
  fi

  printf '%s\n' "${first}"
  printf '# Generated from: graysurf/zsh-kit@%s\n' "${resolved_zsh_kit_ref}"
  printf '# DO NOT EDIT: regenerate via scripts/generate_codex_workspace_bundle.sh\n'

  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "# Bundled from:"* ]]; then
      printf '# Bundled from: %s\n' "${normalized_bundled_from}"
      continue
    fi
    printf '%s\n' "$line"
  done
} <"$tmp_output" >"$tmp_output2"

mkdir -p -- "${output%/*}"
mv -f -- "$tmp_output2" "$output"
chmod +x "$output"

echo "ok: wrote $output (zsh-kit ref: $resolved_zsh_kit_ref)" >&2
