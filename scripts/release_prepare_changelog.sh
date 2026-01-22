#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage:
  scripts/release_prepare_changelog.sh --version <vX.Y.Z> [--date <YYYY-MM-DD>]

what it does:
  - Reads pinned upstream refs from VERSIONS.env (ZSH_KIT_REF + CODEX_KIT_REF).
  - Moves the current CHANGELOG.md "## Unreleased" content into a new release entry:
      ## vX.Y.Z - YYYY-MM-DD
      ### Upstream pins
      - zsh-kit: <ZSH_KIT_REF>
      - codex-kit: <CODEX_KIT_REF>
  - Resets "## Unreleased" to an empty section.

notes:
  - This edits CHANGELOG.md in-place.
  - This does not create commits or tags.
EOF
}

die() {
  echo "release_prepare_changelog: $*" >&2
  exit 2
}

main() {
  local version=''
  local date_str=''
  local changelog='CHANGELOG.md'
  local versions_file='VERSIONS.env'

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
      --date)
        date_str="${2-}"
        shift 2
        ;;
      *)
        die "unknown argument: ${1-}"
        ;;
    esac
  done

  [[ -n "$version" ]] || die "missing --version (expected vX.Y.Z)"
  if [[ -z "$date_str" ]]; then
    date_str="$(date +%Y-%m-%d 2>/dev/null || true)"
  fi
  [[ -n "$date_str" ]] || die "unable to determine --date"

  command -v python3 >/dev/null 2>&1 || die "python3 is required"
  [[ -f "$changelog" ]] || die "missing file: $changelog"
  [[ -f "$versions_file" ]] || die "missing file: $versions_file"

  VERSION="$version" DATE="$date_str" CHANGELOG="$changelog" VERSIONS_FILE="$versions_file" python3 - <<'PY'
import os
import re
import sys
from pathlib import Path


def die(msg: str) -> None:
    print(f"release_prepare_changelog: {msg}", file=sys.stderr)
    raise SystemExit(2)


def _read_versions(path: Path) -> tuple[str, str]:
    zsh_ref: str | None = None
    codex_ref: str | None = None
    for raw in path.read_text("utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("ZSH_KIT_REF="):
            zsh_ref = line.split("=", 1)[1].strip().strip('"').strip("'")
        if line.startswith("CODEX_KIT_REF="):
            codex_ref = line.split("=", 1)[1].strip().strip('"').strip("'")
    if not zsh_ref:
        die(f"VERSIONS.env missing ZSH_KIT_REF: {path}")
    if not codex_ref:
        die(f"VERSIONS.env missing CODEX_KIT_REF: {path}")
    return zsh_ref, codex_ref


def _ensure_release_sections(text: str) -> str:
    return text.strip("\n") + ("\n" if text.strip("\n") else "")


def main() -> None:
    version = os.environ.get("VERSION", "").strip()
    date_str = os.environ.get("DATE", "").strip()
    changelog_path = Path(os.environ.get("CHANGELOG", "CHANGELOG.md"))
    versions_path = Path(os.environ.get("VERSIONS_FILE", "VERSIONS.env"))

    if not version:
        die("missing VERSION")
    if not date_str:
        die("missing DATE")
    if not changelog_path.is_file():
        die(f"missing CHANGELOG: {changelog_path}")
    if not versions_path.is_file():
        die(f"missing VERSIONS.env: {versions_path}")

    zsh_ref, codex_ref = _read_versions(versions_path)
    raw = changelog_path.read_text("utf-8").replace("\r\n", "\n").replace("\r", "\n")

    if f"## {version} - " in raw:
        die(f"CHANGELOG already contains release heading for {version}")

    unreleased_match = re.search(r"(?m)^## Unreleased[ \t]*$", raw)
    if not unreleased_match:
        die("missing '## Unreleased' section in CHANGELOG.md")

    after_unreleased = raw[unreleased_match.end() :]
    next_heading = re.search(r"(?m)^## [^\n]+$", after_unreleased)
    if not next_heading:
        die("unable to find the first release heading after '## Unreleased'")

    unreleased_body = after_unreleased[: next_heading.start()]
    rest = after_unreleased[next_heading.start() :]

    moved = unreleased_body.strip("\n")
    if not moved.strip():
        moved = ""
    moved = _ensure_release_sections(moved)

    release_entry = "\n".join(
        [
            f"## {version} - {date_str}",
            "",
            "### Upstream pins",
            f"- zsh-kit: {zsh_ref}",
            f"- codex-kit: {codex_ref}",
            "",
            moved.strip("\n"),
            "",
        ]
    )

    prefix = raw[: unreleased_match.start()]
    out = (
        prefix.rstrip("\n")
        + "\n\n## Unreleased\n\n"
        + release_entry.rstrip("\n")
        + "\n\n"
        + rest.lstrip("\n").rstrip("\n")
        + "\n"
    )

    changelog_path.write_text(out, "utf-8")
    print(
        f"updated {changelog_path} for {version} (zsh-kit={zsh_ref} codex-kit={codex_ref})"
    )


if __name__ == "__main__":
    main()
PY
}

main "$@"
