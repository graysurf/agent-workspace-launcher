# Agent Workspace Rust Contract

## Purpose

Define the Rust `agent-workspace-launcher` contract implemented in this repository.

## Commands

- `agent-workspace-launcher auth ...`
- `agent-workspace-launcher create ...`
- `agent-workspace-launcher ls ...`
- `agent-workspace-launcher rm ...`
- `agent-workspace-launcher exec ...`
- `agent-workspace-launcher reset ...`
- `agent-workspace-launcher tunnel ...`

Alias contract:

- `awl ...` must resolve to the same parser and dispatch behavior.

## Runtime behavior

- Rust CLI is the runtime implementation, not a Docker wrapper.
- Subcommands execute host-native workspace operations.
- Workspace root resolution order:
  1. `AGENT_WORKSPACE_HOME`
  2. `XDG_STATE_HOME/agent-workspace-launcher/workspaces`
  3. `$HOME/.local/state/agent-workspace-launcher/workspaces`
- Exit codes reflect runtime result directly (`0` success, non-zero failure).

## Naming policy

- Canonical binary name: `agent-workspace-launcher`.
- Compatibility alias: `awl`.
- No `cws` shim and no `CWS_*` fallback.

## Codex compatibility exceptions

These env names are intentionally preserved:

- `CODEX_SECRET_DIR`
- `CODEX_AUTH_FILE`
