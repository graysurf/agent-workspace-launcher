# Design: agent-workspace-launcher

## Goal

Provide a host-native workspace lifecycle CLI that does not depend on a container backend for normal operation.

## Runtime architecture

Primary command path:

1. User invokes `agent-workspace-launcher` (or alias `awl`).
2. Rust CLI parses subcommands and executes host-native handlers.
3. Workspace state is represented on host filesystem (no Docker workspace container lifecycle).

## Command surface

- `auth`
- `create`
- `ls`
- `rm`
- `exec`
- `reset`
- `tunnel`

## Storage model

Workspace root resolution order:

1. `AGENT_WORKSPACE_HOME`
2. `XDG_STATE_HOME/agent-workspace-launcher/workspaces`
3. `$HOME/.local/state/agent-workspace-launcher/workspaces`

Each workspace is a directory with subpaths such as `work/`, `private/`, `opt/`, `auth/`, `.codex/`.

## Auth model

- GitHub auth prefers host `gh` keyring or `GH_TOKEN` / `GITHUB_TOKEN` (policy via `AGENT_WORKSPACE_AUTH`).
- Codex auth keeps compatibility names: `CODEX_SECRET_DIR`, `CODEX_AUTH_FILE`.
- GPG auth stores selected key metadata in workspace auth state.

## Compatibility

- `awl` is an alias compatibility layer only.
- `agent-workspace-launcher` is the canonical command identity for release assets and docs.

## Packaging direction

- CLI assets are primary distribution artifacts.
- Docker packaging may exist as a compatibility channel but is not the required runtime backend.
