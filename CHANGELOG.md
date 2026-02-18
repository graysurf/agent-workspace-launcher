# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Changed
- Add dual runtime support to the Rust CLI (`container` default, explicit `host` fallback) with unified runtime resolution:
  `--runtime` > `AGENT_WORKSPACE_RUNTIME` > `AWL_RUNTIME` > default `container`.
- Add container backend handlers for `create`, `ls`, `exec`, `rm`, `reset`, `auth`, and `tunnel`.
- Keep host backend behavior available via explicit runtime selection.

### Docs
- Update architecture/spec/guide contracts from host-only wording to dual runtime (`container` default, `host` fallback) while keeping subcommand surface unchanged.
- Document runtime selectors and related env/flags: `--runtime`, `AGENT_WORKSPACE_RUNTIME`, `AWL_RUNTIME`, `AGENT_ENV_IMAGE`, `CODEX_ENV_IMAGE`.
- Expand parity and integration runbooks to require both default-container smoke and explicit host-fallback smoke.

## v1.1.3 - 2026-02-17

### Upstream pins
- agent-kit: 0ae695f18f672bf1f418c068a5e3124a26ecfb1a

### Fixed
- Align CLI `--version` / `-V` output with release tags for packaged binaries.
- Inject release semver into Homebrew and Docker release build pipelines.

## v1.1.2 - 2026-02-17

### Upstream pins
- agent-kit: 0ae695f18f672bf1f418c068a5e3124a26ecfb1a

### Changed
- Package bash/zsh completion files in `release-brew` archives for Homebrew formula installs.
- Add a release-homebrew post-tap verification script to ensure local `agent-workspace-launcher` and `awl`
  are upgraded to the target version.

### Docs
- Document the required post-release local Homebrew verification flow and version checks.

## v1.1.1 - 2026-02-16

### Upstream pins
- agent-kit: 0ae695f18f672bf1f418c068a5e3124a26ecfb1a

## v1.1.0 - 2026-02-17

### Upstream pins
- agent-kit: 0ae695f18f672bf1f418c068a5e3124a26ecfb1a

### Changed
- Split release automation into tag-driven Docker and Brew channels.
- Pin launcher image builds to `agent-kit` commit `0ae695f18f672bf1f418c068a5e3124a26ecfb1a`.

### Fixed
- Normalize `ws-` workspace aliases during container resolution so `aws exec --user/--root` targets existing containers.

## v1.0.7 - 2026-02-09

### Upstream pins
- zsh-kit: dcf65f9600c24b87d14bf506d017996a70a32103
- agent-kit: f492fd8b78c2068995baa953c6daaf2369e7246c

## v1.0.6 - 2026-01-25

### Upstream pins
- zsh-kit: dcf65f9600c24b87d14bf506d017996a70a32103
- agent-kit: f492fd8b78c2068995baa953c6daaf2369e7246c

### Changed
- Bump pinned upstream ref (agent-kit; zsh-kit unchanged).

## v1.0.5 - 2026-01-24

### Upstream pins
- zsh-kit: dcf65f9600c24b87d14bf506d017996a70a32103
- agent-kit: 3aae1c3cbc4ca33d0d0656b0bef457ea19766b5b

### Changed
- Bump pinned upstream refs (zsh-kit + agent-kit).

## v1.0.4 - 2026-01-22

### Upstream pins
- zsh-kit: bbc89ec80659df8b76e8c98f44f510c14d34ea54
- agent-kit: a3d7eb40d9a895546d60041b5d8ac850a7b03933

### Changed
- Remove `--persist-gh-token` flag (auth is now applied container-side).
- Align launcher `create` parameters with updated zsh-kit behavior.

### Docs
- Remove outdated security note from `docs/DESIGN.md`.

## v1.0.3 - 2026-01-22

### Upstream pins
- zsh-kit: aa964753efcea4466ee7789151eb81083ebc4d11
- agent-kit: c244ea723abce70fc9045828f8b6c785bc597cce

### Fixed
- Install `jq` in the launcher image so `cws create` can parse launcher JSON and print the Dev Containers VS Code link.

## v1.0.2 - 2026-01-22

### Upstream pins
- zsh-kit: aa964753efcea4466ee7789151eb81083ebc4d11
- agent-kit: c244ea723abce70fc9045828f8b6c785bc597cce

### Changed
- Remove `scripts/cws` shim; use `scripts/cws.bash` directly for executable mode.

## v1.0.1 - 2026-01-21

### Added
- Docker-backed e2e suite for `cws` (CLI plan matrix + wrapper flow tests), gated behind `AWL_E2E=1`.
- Script smoke testing infrastructure with stubbed `docker`, plus wrapper equivalence tests for `scripts/cws.bash` and `scripts/cws.zsh`.
- `cws auth` command for refreshing GitHub/Codex/GPG credentials inside a workspace.
- Optional GPG signing key injection support (`AGENT_WORKSPACE_GPG` / `AGENT_WORKSPACE_GPG_KEY`).
- Pyright type checking + Ruff lint/format configuration; documented required pre-submit checks.

### Changed
- `scripts/cws` now delegates to `scripts/cws.bash` and aligns bash/zsh wrapper behavior.

### Fixed
- Zsh completion argument positions for `cws` subcommands.

## v1.0.0 - 2026-01-20

### Added
- Portable Docker launcher image for `agent-workspace` (Docker-outside-of-Docker / DooD)
- Optional `cws` wrappers (zsh + bash + completion) to run the launcher image
- Documentation: quickstart, build notes, troubleshooting, integration test runbook, and a multi-page user guide
- CI publishing to Docker Hub and GitHub Container Registry (GHCR)

### Changed
- README and docs tuned for end users (Codex-ready workspace, copy/paste-first)

### Fixed
- Documentation fixes: correct relative links and note the `exec --` gotcha

### Known
- `agent-workspace exec <name> -- <cmd>` is not supported in the current launcher image (it tries to run `--`)
