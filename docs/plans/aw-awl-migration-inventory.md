# AW/AWL Migration Inventory

## Runtime
- Added Rust workspace and crate: `crates/agent-workspace/*`
- Docker entrypoint switched to Rust binary (`Dockerfile`)
- Removed zsh bundle path files:
  - `bin/agent-workspace`
  - `scripts/generate_agent_workspace_bundle.sh`
  - `scripts/bundles/agent-workspace.wrapper.zsh`

## Host wrappers
- Added: `scripts/awl.bash`, `scripts/awl.zsh`
- Removed: `scripts/cws.bash`, `scripts/cws.zsh`

## Tests
- Renamed e2e modules to `test_awl_*`
- Renamed script specs to `awl.*.json`
- Removed cws specs and bundle-generation specs

## CI / Release
- CI adds Rust fmt/check/clippy/test
- Publish pipeline consumes only `AGENT_KIT_REF`
- Release scripts and version bumping remove zsh bundle assumptions

## Docs
- `docs/guides/cws/*` moved to `docs/guides/awl/*`
- User docs switched to `awl` and `AWL_*`
- Codex naming exception documented: `CODEX_SECRET_DIR`, `CODEX_AUTH_FILE`
