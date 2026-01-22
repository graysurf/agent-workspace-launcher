# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Changed
- Remove `scripts/cws` shim; use `scripts/cws.bash` directly for executable mode.

## v1.0.1 - 2026-01-21

### Added
- Docker-backed e2e suite for `cws` (CLI plan matrix + wrapper flow tests), gated behind `CWS_E2E=1`.
- Script smoke testing infrastructure with stubbed `docker`, plus wrapper equivalence tests for `scripts/cws.bash` and `scripts/cws.zsh`.
- `cws auth` command for refreshing GitHub/Codex/GPG credentials inside a workspace.
- Optional GPG signing key injection support (`CODEX_WORKSPACE_GPG` / `CODEX_WORKSPACE_GPG_KEY`).
- Pyright type checking + Ruff lint/format configuration; documented required pre-submit checks.

### Changed
- `scripts/cws` now delegates to `scripts/cws.bash` and aligns bash/zsh wrapper behavior.

### Fixed
- Zsh completion argument positions for `cws` subcommands.

## v1.0.0 - 2026-01-20

### Added
- Portable Docker launcher image for `codex-workspace` (Docker-outside-of-Docker / DooD)
- Optional `cws` wrappers (zsh + bash + completion) to run the launcher image
- Documentation: quickstart, build notes, troubleshooting, integration test runbook, and a multi-page user guide
- CI publishing to Docker Hub and GitHub Container Registry (GHCR)

### Changed
- README and docs tuned for end users (Codex-ready workspace, copy/paste-first)

### Fixed
- Documentation fixes: correct relative links and note the `exec --` gotcha

### Known
- `codex-workspace exec <name> -- <cmd>` is not supported in the current launcher image (it tries to run `--`)
