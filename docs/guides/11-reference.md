# Reference

## Commands

| Command | Purpose |
| --- | --- |
| `agent-workspace-launcher --help` | Show help |
| `agent-workspace-launcher --runtime container <subcommand> ...` | Force container backend |
| `agent-workspace-launcher --runtime host <subcommand> ...` | Force host backend |
| `agent-workspace-launcher create ...` | Create workspace |
| `agent-workspace-launcher ls` | List workspaces |
| `agent-workspace-launcher exec ...` | Run command/shell in workspace |
| `agent-workspace-launcher rm ...` | Remove workspace(s) |
| `agent-workspace-launcher reset ...` | Reset repos in workspace |
| `agent-workspace-launcher auth ...` | Update auth material |
| `agent-workspace-launcher tunnel ...` | Start VS Code tunnel |
| `agent-workspace-launcher __complete ...` | Internal completion protocol endpoint (shell adapters use this) |
| `awl ...` | Alias compatibility form |

Runtime notes:

- Runtime resolution: `--runtime` > `AGENT_WORKSPACE_RUNTIME` > `AWL_RUNTIME` > default `container`.
- Command tree is unchanged across runtimes.
- Completion runtime resolution uses the same precedence as command execution.

## Environment

| Env | Default | Purpose |
| --- | --- | --- |
| `AGENT_WORKSPACE_RUNTIME` | `container` | Runtime backend selector (`container\|host`) |
| `AWL_RUNTIME` | (empty) | Compatibility runtime selector alias |
| `AGENT_WORKSPACE_COMPLETION_MODE` | `rust` | Completion mode: `rust` (default) or `legacy` (rollback) |
| `AGENT_WORKSPACE_HOME` | auto | Workspace root override |
| `AGENT_WORKSPACE_PREFIX` | `agent-ws` | Workspace prefix normalization |
| `AGENT_WORKSPACE_AUTH` | `auto` | GitHub token source policy |
| `AGENT_WORKSPACE_GPG_KEY` | (empty) | Default key for `auth gpg` |
| `AGENT_ENV_IMAGE` | `graysurf/agent-env:latest` | Container runtime `create` image default |
| `CODEX_ENV_IMAGE` | (empty) | Compatibility fallback for container image |
| `CODEX_SECRET_DIR` | (empty) | Codex profile directory (compat) |
| `CODEX_AUTH_FILE` | `~/.codex/auth.json` | Codex auth file path (compat) |

Container-only option references:

- `create --image <image>`: override container image for one command.
- `create --no-pull`: require image to exist locally.
- `rm --keep-volumes`: preserve workspace volumes during container removal.

## Completion protocol (internal)

- Hidden command shape:
  - `agent-workspace-launcher __complete --shell <bash|zsh> --cword <index> --word <arg0> --word <arg1> ...`
  - Optional: `--format describe` to emit `candidate<TAB>description` lines.
- Adapters forward shell state to this command and consume newline-delimited candidates.
- Runtime-aware workspace candidates apply to `auth`, `rm`, `exec`, `reset`, and `tunnel`.
- Rollback toggle:
  - `export AGENT_WORKSPACE_COMPLETION_MODE=legacy`
