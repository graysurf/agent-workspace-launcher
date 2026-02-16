---
name: release-agent-workspace-launcher
description: "Release agent-workspace-launcher: enforce native CLI checks, keep name contract (agent-workspace-launcher primary / awl alias), and publish tag-based CLI archives."
---

# Release Workflow (agent-workspace-launcher)

This repo releases from semver tags (`vX.Y.Z`).
Primary release output is native CLI archives for Homebrew/manual install.

## Project-specific non-negotiables

1) `agent-workspace-launcher` is the canonical command identity in release assets/docs.
2) `awl` is compatibility alias only (same binary behavior).
3) Release readiness is gated by required repo checks (`DEVELOPMENT.md`) and CLI archive verification.

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

Stop conditions:

- Any required check fails.
- Changelog audit fails.
- Release asset verification fails (missing targets, checksum mismatch, missing alias payload).

## Workflow

1. Decide version + date
   - Version: `vX.Y.Z`
   - Date: `YYYY-MM-DD` (default: `date +%Y-%m-%d`)

2. Run required repo checks (per `DEVELOPMENT.md`)
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

3. Local binary smoke
   - `cargo build --release -p agent-workspace --bin agent-workspace-launcher`
   - `./target/release/agent-workspace-launcher --help`
   - `tmp="$(mktemp -d)"; ln -sf "$(pwd)/target/release/agent-workspace-launcher" "$tmp/awl"; "$tmp/awl" --help`

4. Prepare changelog
   - `./scripts/release_prepare_changelog.sh --version vX.Y.Z`

5. Commit release notes
   - Suggested message: `chore(release): vX.Y.Z`
   - Use semantic commit helper (do not call `git commit` directly).

6. Audit (strict)
   - `./scripts/release_audit.sh --version vX.Y.Z --branch main --strict`

7. Tag and push
   - `git -c tag.gpgSign=false tag vX.Y.Z`
   - `git push origin vX.Y.Z`

8. Verify CLI channel
   - Confirm `release-brew.yml` ran for `vX.Y.Z`
   - Confirm release assets include all target tarballs + checksums
   - Confirm tarball payload includes both `bin/agent-workspace-launcher` and `bin/awl`

## Helper scripts (project)

- Prepare changelog: `scripts/release_prepare_changelog.sh`
- Audit release entry: `scripts/release_audit.sh`

## Optional compatibility channel

Container-image publishing may remain as optional compatibility work, but it must not gate native CLI release completion.

## Output templates

- Success: `.agents/skills/release-workflow/references/OUTPUT_TEMPLATE.md`
- Blocked: `.agents/skills/release-workflow/references/OUTPUT_TEMPLATE_BLOCKED.md`
