# Agent Workspace Parity Matrix

## Areas

- Command behavior parity between `agent-workspace-launcher` and `awl` alias
- Runtime routing parity (`container` default, explicit `host` fallback)
- Wrapper parity (`scripts/awl.bash` vs `scripts/awl.zsh`)
- Container backend parity (`create|ls|exec|rm|reset|auth|tunnel`)
- Host backend parity (`create|ls|exec|rm|reset|auth|tunnel`)
- Auth behavior parity (`AGENT_WORKSPACE_AUTH`, `GH_TOKEN`/`GITHUB_TOKEN`)
- Codex env compatibility (`CODEX_SECRET_DIR`, `CODEX_AUTH_FILE`)

## Validation set

- Rust unit/integration:
  - `cargo test -p agent-workspace`
- Runtime resolver behavior:
  - `cargo test -p agent-workspace runtime::tests`
- Script smoke:
  - `.venv/bin/python -m pytest -m script_smoke`
- Wrapper equivalence:
  - `.venv/bin/python -m pytest tests/test_wrapper_equivalence.py`
- Runtime smoke matrix (CLI behavior):
  - `AWL_E2E=1 .venv/bin/python -m pytest -m e2e tests/e2e/test_awl_cli_cases.py`

## Release payload parity checks

- `release-brew.yml` assets contain:
  - `bin/agent-workspace-launcher`
  - `bin/awl`
- Homebrew install exposes both command names and both return help output.
- Help output exposes runtime controls (`--runtime <container|host>`).

## Out of scope for parity gate

- `awl_docker` wrapper-specific environment toggles (`AWL_DOCKER_*`).
- Legacy `cws` compatibility paths.
