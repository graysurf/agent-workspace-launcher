# Agent Workspace Parity Matrix

## Areas
- Command forwarding parity: `auth/create/ls/rm/exec/reset/tunnel`
- Host wrapper docker argv parity (`aws.bash` vs `aws.zsh`)
- Auth behavior parity (`GH_TOKEN`/`GITHUB_TOKEN` forwarding, `AWS_AUTH` policy)
- E2E gating parity (`AWS_E2E_*` matrix)

## Validation set
- Rust unit/integration:
  - `cargo test -p agent-workspace`
- Script smoke:
  - `.venv/bin/python -m pytest -m script_smoke`
- Wrapper equivalence:
  - `.venv/bin/python -m pytest tests/test_wrapper_equivalence.py`
- E2E (smoke case):
  - `AWS_E2E=1 AWS_E2E_CASE=help .venv/bin/python -m pytest -m e2e tests/e2e/test_aws_cli_cases.py`

## Agent-env integration checks
- Launcher path resolution auto-detects `/opt/agent-kit/docker/agent-env/bin/agent-workspace`.
- `create` emits expected workspace/path outputs through low-level launcher.
- `exec/reset/rm` round-trips through wrapper and low-level launcher contract.

## Codex env compatibility checks
- `CODEX_SECRET_DIR` is passed through unchanged.
- `CODEX_AUTH_FILE` is passed through unchanged.
