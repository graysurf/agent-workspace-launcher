# Agent Workspace Rust Contract

## Purpose
Define the Rust `agent-workspace` CLI contract shipped in the launcher image.

## Commands
- `agent-workspace auth ...`
- `agent-workspace create ...`
- `agent-workspace ls ...`
- `agent-workspace rm ...`
- `agent-workspace exec ...`
- `agent-workspace reset ...`
- `agent-workspace tunnel ...`

## Runtime behavior
- Rust CLI is a front controller.
- Each subcommand forwards args to low-level launcher at:
  - Env override: `AGENT_WORKSPACE_LAUNCHER`
  - Auto-detect default:
    - `/opt/agent-kit/docker/agent-env/bin/agent-workspace`
- Exit code mirrors child process exit code.

## Naming policy
- Host wrapper naming is `aws` and `aw*` shorthand.
- Host env naming is `AWS_*`.
- No `cws` shim and no `CWS_*` runtime fallback.

## Codex exceptions (unchanged)
- `CODEX_SECRET_DIR`
- `CODEX_AUTH_FILE`

These two names are intentionally preserved for Codex auth compatibility.
