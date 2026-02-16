#!/usr/bin/env bash
set -euo pipefail

die() {
  echo "release-notes-from-changelog: $1" >&2
  exit 2
}

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/release_notes_from_changelog.sh --version <vX.Y.Z> [--changelog <path>] [--output <path>]

behavior:
  - Extracts the matching "## <version> ..." section from a changelog until the next "## " heading.
  - Writes the extracted notes to --output.
  - Default output (when AGENT_HOME is set): $AGENT_HOME/out/release-notes-<version>.md

notes:
  - Default changelog path: CHANGELOG.md
EOF
}

version=''
changelog='CHANGELOG.md'
output=''

while [[ $# -gt 0 ]]; do
  case "${1:-}" in
    --version)
      version="${2:-}"
      shift 2
      ;;
    --changelog)
      changelog="${2:-}"
      shift 2
      ;;
    --output)
      output="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: ${1:-}"
      ;;
  esac
done

[[ -n "$version" ]] || die "missing --version (expected vX.Y.Z)"
[[ -f "$changelog" ]] || die "changelog not found: $changelog"

if [[ -z "$output" ]]; then
  if [[ -n "${AGENT_HOME:-}" ]]; then
    output="${AGENT_HOME%/}/out/release-notes-${version}.md"
  else
    output="./release-notes-${version}.md"
  fi
fi

mkdir -p -- "$(dirname "$output")"

awk -v v="$version" '
  $0 ~ "^## " v " " { f=1; heading=NR }
  f {
    if (NR > heading && $0 ~ "^## ") { exit }
    print
  }
' "$changelog" >"$output"

if [[ ! -s "$output" ]]; then
  rm -f -- "$output"
  die "version section not found in $changelog: $version"
fi

echo "$output"
