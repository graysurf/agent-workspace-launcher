# Design: agent-workspace-launcher

## Goal

Provide a workspace lifecycle CLI with dual runtime backends:

- `container` runtime (default)
- `host` runtime (explicit fallback)

The command surface remains stable across both runtimes.

## Runtime architecture

Primary command path:

1. User invokes `agent-workspace-launcher` (or alias `awl`).
2. Rust CLI parses subcommands (including hidden internal `__complete`) and preserves existing public command tree.
3. For public subcommands, runtime resolver selects backend using this precedence:
   1. `--runtime container|host`
   2. `AGENT_WORKSPACE_RUNTIME`
   3. `AWL_RUNTIME` (compat env alias)
   4. default `container`
4. Runtime dispatch executes backend-specific handlers:
   - `container`: Docker-backed workspace lifecycle.
   - `host`: host-filesystem workspace lifecycle.

## Completion architecture

- Internal endpoint: `__complete` is a hidden Rust subcommand used by shell completion adapters.
- Adapter model: bash/zsh completion files and wrapper scripts are thin adapters that send shell context to `__complete` and render returned candidates.
- Runtime-aware completion: workspace suggestions for `auth`, `rm`, `exec`, `reset`, and `tunnel` are resolved against the selected runtime backend using the same precedence as normal execution.
- Rollback mode: `AGENT_WORKSPACE_COMPLETION_MODE=legacy` forces adapters to use legacy shell completion logic instead of Rust-backed completion.

## Command surface

- `auth`
- `create`
- `ls`
- `rm`
- `exec`
- `reset`
- `tunnel`

## State model

Container runtime:

- Workspaces are Docker containers (name-normalized by workspace prefix rules).
- Default workspace image: `graysurf/agent-env:latest`.
- Image override sources (highest to lowest):
  1. `create --image <image>`
  2. `AGENT_ENV_IMAGE`
  3. `CODEX_ENV_IMAGE`
- Workspace data volumes are runtime-managed per workspace.

Host runtime:

Workspace root resolution order:

1. `AGENT_WORKSPACE_HOME`
2. `XDG_STATE_HOME/agent-workspace-launcher/workspaces`
3. `$HOME/.local/state/agent-workspace-launcher/workspaces`

Each workspace is a directory with subpaths such as `work/`, `private/`, `opt/`, `auth/`, `.codex/`.

## Auth model

- GitHub auth prefers host `gh` keyring or `GH_TOKEN` / `GITHUB_TOKEN` (policy via `AGENT_WORKSPACE_AUTH`).
- Codex auth keeps compatibility names: `CODEX_SECRET_DIR`, `CODEX_AUTH_FILE`.
- GPG auth stores selected key metadata in workspace auth state.
- Runtime behavior differs by backend target:
  - `container`: writes auth material into workspace container filesystem.
  - `host`: writes auth material into host workspace directories.

## Compatibility

- `awl` is an alias compatibility layer only.
- `agent-workspace-launcher` is the canonical command identity for release assets and docs.
- Runtime selection is additive and does not change command names or subcommands.

## Failure behavior

- If the selected/default runtime is `container` and Docker is unavailable, runtime exits non-zero with host-fallback guidance (`--runtime host` or `AGENT_WORKSPACE_RUNTIME=host`).
- Invalid runtime values fail fast with explicit `container|host` expectation messaging.
- Completion failures should degrade gracefully (prefer partial/static candidates over shell-breaking errors).
- Operational rollback is environment-only: set `AGENT_WORKSPACE_COMPLETION_MODE=legacy` and reload the shell.

## Packaging direction

- CLI assets and Docker image are both supported distribution channels.
- Runtime default is container-backed; host runtime remains supported for fallback/local use cases.
