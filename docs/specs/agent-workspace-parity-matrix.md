# Agent Workspace Parity Matrix

## Areas

- Command behavior parity between `agent-workspace-launcher` and `awl` alias
- Wrapper parity (`scripts/awl.bash` vs `scripts/awl.zsh`)
- Auth behavior parity (`AGENT_WORKSPACE_AUTH`, `GH_TOKEN`/`GITHUB_TOKEN`)
- Codex env compatibility (`CODEX_SECRET_DIR`, `CODEX_AUTH_FILE`)

## Validation set

- Rust unit/integration:
  - `cargo test -p agent-workspace`
- Script smoke:
  - `.venv/bin/python -m pytest -m script_smoke`
- Wrapper equivalence:
  - `.venv/bin/python -m pytest tests/test_wrapper_equivalence.py`

## Release payload parity checks

- `release-brew.yml` assets contain:
  - `bin/agent-workspace-launcher`
  - `bin/awl`
- Homebrew install exposes both command names and both return help output.

## Out of scope for parity gate

- Container-backend behavior (`docker run`, `AWL_IMAGE`, `AWL_DOCKER_ARGS`)
- Legacy `cws` compatibility paths
