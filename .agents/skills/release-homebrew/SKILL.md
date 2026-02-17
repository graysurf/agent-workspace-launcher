---
name: release-homebrew
description: "Release Homebrew channel for agent-workspace-launcher: run required checks, cut vX.Y.Z tag, and verify release-brew assets."
---

# Release Homebrew (agent-workspace-launcher)

This repo releases from semver tags (`vX.Y.Z`).
Primary release output is native CLI archives for Homebrew/manual install.

## Contract

Prereqs:

- Run in repo root.
- Working tree clean before tagging.
- `git` available on `PATH`.
- Recommended: `gh auth status` succeeds.

Inputs:

- Release version: `vX.Y.Z`
- Optional release date: `YYYY-MM-DD`

Outputs:

- `CHANGELOG.md` updated for `vX.Y.Z`
- Required checks passed
- Tag `vX.Y.Z` pushed
- `release-brew.yml` published assets + checksums for all targets

Exit codes:

- `0`: success
- `1`: failure
- `2`: usage error

Failure modes:

- Any required check fails.
- Changelog audit fails.
- Release asset verification fails (missing targets, checksum mismatch, missing alias/completion payload).
- Local Homebrew validation fails (formula still old, PATH still resolving old `awl`, or completion files missing).
- Missing/invalid release version input.

## Scripts (only entrypoints)

- `<PROJECT_ROOT>/.agents/skills/release-homebrew/scripts/release-homebrew.sh`
- `<PROJECT_ROOT>/.agents/skills/release-homebrew/scripts/verify-brew-installed-version.sh`

## Workflow

1. Run the skill entrypoint:
   - `.agents/skills/release-homebrew/scripts/release-homebrew.sh --version vX.Y.Z [--date YYYY-MM-DD]`

2. Decide version + date
   - Version: `vX.Y.Z`
   - Date: `YYYY-MM-DD` (default: `date +%Y-%m-%d`)

3. Run required repo checks (per `DEVELOPMENT.md`)
   - `bash -n $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
   - `zsh -n $(git ls-files 'scripts/*.zsh')`
   - `shellcheck $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
   - `.venv/bin/python -m ruff format --check .`
   - `.venv/bin/python -m ruff check .`
   - `.venv/bin/python -m pytest -m script_smoke`
   - `cargo fmt --all -- --check`
   - `cargo check --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test -p agent-workspace`

4. Local binary smoke
   - `cargo build --release -p agent-workspace --bin agent-workspace-launcher`
   - `./target/release/agent-workspace-launcher --help`
   - `tmp="$(mktemp -d)"; ln -sf "$(pwd)/target/release/agent-workspace-launcher" "$tmp/awl"; "$tmp/awl" --help`

5. Prepare changelog
   - `./scripts/release_prepare_changelog.sh --version vX.Y.Z`

6. Commit release notes
   - Suggested message: `chore(release): vX.Y.Z`
   - Use semantic commit helper (do not call `git commit` directly).

7. Audit (strict)
   - `./scripts/release_audit.sh --version vX.Y.Z --branch main --strict`

8. Tag and push
   - `git -c tag.gpgSign=false tag vX.Y.Z`
   - `git push origin vX.Y.Z`

9. Verify CLI channel
   - Confirm `release-brew.yml` ran for `vX.Y.Z`
   - Confirm release assets include all target tarballs + checksums
   - Confirm tarball payload includes:
     - `bin/agent-workspace-launcher`
     - `bin/awl`
     - `completions/agent-workspace-launcher.bash`
     - `completions/_agent-workspace-launcher`

10. Verify local Homebrew is upgraded to the same version
   - Run after `homebrew-tap` formula update/merge:
   - `.agents/skills/release-homebrew/scripts/verify-brew-installed-version.sh --version vX.Y.Z`
   - If validating against a local tap checkout:
   - `.agents/skills/release-homebrew/scripts/verify-brew-installed-version.sh --version vX.Y.Z --tap-repo ~/Project/graysurf/homebrew-tap`

## Project-specific non-negotiables

1) `agent-workspace-launcher` is the canonical command identity in release assets/docs.
2) `awl` is compatibility alias only (same binary behavior).
3) Release readiness is gated by required repo checks (`DEVELOPMENT.md`) and CLI archive verification.

## Optional compatibility channel

Container-image publishing may remain as optional compatibility work, but it must not gate native CLI release completion.

## Output templates

- Success: `.agents/skills/release-homebrew/references/OUTPUT_TEMPLATE.md`
- Blocked: `.agents/skills/release-homebrew/references/OUTPUT_TEMPLATE_BLOCKED.md`
