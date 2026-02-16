#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
usage:
  scripts/generate_agent_workspace_bundle.sh

notes:
  - Regenerates ./bin/agent-workspace from the pinned ZSH_KIT_REF in VERSIONS.env.
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
  tmpdir="/tmp/agent-workspace-launcher.bundle.$$"
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

manifest="${repo_root}/scripts/bundles/agent-workspace.wrapper.zsh"
if [[ ! -f "$manifest" ]]; then
  echo "error: missing bundle manifest: $manifest" >&2
  exit 1
fi

feature_namespace='agent-workspace'
if [[ ! -f "${zsh_kit_dir}/scripts/_features/agent-workspace/alias.zsh" ]]; then
  if [[ -f "${zsh_kit_dir}/scripts/_features/codex-workspace/alias.zsh" ]]; then
    feature_namespace='codex-workspace'
  else
    echo "error: missing expected zsh-kit feature namespace: agent-workspace (or codex-workspace fallback)" >&2
    echo "hint: check ZSH_KIT_REF=$ZSH_KIT_REF" >&2
    exit 1
  fi
fi

manifest_input="$manifest"
if [[ "$feature_namespace" == "codex-workspace" ]]; then
  manifest_input="${tmpdir}/agent-workspace.wrapper.compat.zsh"
  sed 's#_features/agent-workspace/#_features/codex-workspace/#g' "$manifest" >"$manifest_input"
fi

for feature_file in alias.zsh repo-reset.zsh workspace-rm.zsh workspace-rsync.zsh workspace-launcher.zsh; do
  required="scripts/_features/${feature_namespace}/${feature_file}"
  if [[ ! -f "${zsh_kit_dir}/${required}" ]]; then
    echo "error: missing expected zsh-kit file: ${required}" >&2
    echo "hint: check ZSH_KIT_REF=$ZSH_KIT_REF and ensure it contains ${feature_namespace}" >&2
    exit 1
  fi
done

output="${repo_root}/bin/agent-workspace"
tmp_output="${tmpdir}/agent-workspace.bundled"
tmp_output_transformed="${tmpdir}/agent-workspace.bundled.transformed"
tmp_output_source="$tmp_output"

ZDOTDIR="${zsh_kit_dir}" \
ZSH_CONFIG_DIR="${zsh_kit_dir}/config" \
ZSH_BOOTSTRAP_SCRIPT_DIR="${zsh_kit_dir}/bootstrap" \
ZSH_SCRIPT_DIR="${zsh_kit_dir}/scripts" \
  zsh -f "$bundle_wrapper" \
  --input "$manifest_input" \
  --output "$tmp_output" \
  --entry agent-workspace

normalized_bundled_from="scripts/bundles/agent-workspace.wrapper.zsh"
tmp_output2="${tmpdir}/agent-workspace.bundled.header"

if [[ "$feature_namespace" == "codex-workspace" ]]; then
  # Transitional compatibility:
  # upstream zsh-kit may still ship codex-named feature sources while this repo
  # exposes an agent-facing command/env contract.
  perl -pe '
    s/\bcodex-workspace\b/agent-workspace/g;
    s/\bCODEX_WORKSPACE_/AGENT_WORKSPACE_/g;
    s/\bAGENT_WORKSPACE_CODEX_PROFILE\b/AGENT_WORKSPACE_AGENT_PROFILE/g;
    s/\bCODEX_HOME\b/AGENT_HOME/g;
    s/codex-kit\.workspace/agent-kit.workspace/g;
    s/\bcodex-kit\b/agent-kit/g;
    s/\bcodex-env\b/agent-env/g;
    s/\.agent-env/.agents-env/g;
    s/\bcodex_secrets\b/AGENT_secrets/g;
    s{/home/codex/codex_secrets}{/home/agent/AGENT_secrets}g;
    s{/home/codex}{/home/agent}g;
    s{/home/agent/\.codex}{/home/agent/.agents}g;
    s{\.config/codex_secrets}{.config/AGENT_secrets}g;
  ' "$tmp_output" >"$tmp_output_transformed"
  tmp_output_source="$tmp_output_transformed"
fi

{
  IFS= read -r first || true
  if [[ -z "${first}" ]]; then
    echo "error: bundle output is empty" >&2
    exit 1
  fi

  printf '%s\n' "${first}"
  printf '# Generated from: graysurf/zsh-kit@%s\n' "${resolved_zsh_kit_ref}"
  printf '# DO NOT EDIT: regenerate via scripts/generate_agent_workspace_bundle.sh\n'

  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "# Bundled from:"* ]]; then
      printf '# Bundled from: %s\n' "${normalized_bundled_from}"
      continue
    fi
    printf '%s\n' "$line"
  done
} <"$tmp_output_source" >"$tmp_output2"

mkdir -p -- "${output%/*}"
mv -f -- "$tmp_output2" "$output"
chmod +x "$output"

echo "ok: wrote $output (zsh-kit ref: $resolved_zsh_kit_ref)" >&2
