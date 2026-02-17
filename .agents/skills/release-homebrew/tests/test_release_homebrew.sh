#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
skill_root="$(cd "${script_dir}/.." && pwd)"

if [[ ! -f "${skill_root}/SKILL.md" ]]; then
  echo "error: missing SKILL.md" >&2
  exit 1
fi

entrypoint="${skill_root}/scripts/release-homebrew.sh"
if [[ ! -x "${entrypoint}" ]]; then
  echo "error: missing executable ${entrypoint}" >&2
  exit 1
fi

"${entrypoint}" --help >/dev/null

verify_entrypoint="${skill_root}/scripts/verify-brew-installed-version.sh"
if [[ ! -x "${verify_entrypoint}" ]]; then
  echo "error: missing executable ${verify_entrypoint}" >&2
  exit 1
fi

"${verify_entrypoint}" --help >/dev/null

echo "ok: release-homebrew skill smoke checks passed"
