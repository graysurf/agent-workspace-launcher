# Reference

## Commands

| Command | Purpose |
| --- | --- |
| `agent-workspace-launcher --help` | Show help |
| `agent-workspace-launcher create ...` | Create workspace |
| `agent-workspace-launcher ls` | List workspaces |
| `agent-workspace-launcher exec ...` | Run command/shell in workspace |
| `agent-workspace-launcher rm ...` | Remove workspace(s) |
| `agent-workspace-launcher reset ...` | Reset repos in workspace |
| `agent-workspace-launcher auth ...` | Update auth material |
| `agent-workspace-launcher tunnel ...` | Start VS Code tunnel |
| `awl ...` | Alias compatibility form |

## Environment

| Env | Default | Purpose |
| --- | --- | --- |
| `AGENT_WORKSPACE_HOME` | auto | Workspace root override |
| `AGENT_WORKSPACE_PREFIX` | `agent-ws` | Workspace prefix normalization |
| `AGENT_WORKSPACE_AUTH` | `auto` | GitHub token source policy |
| `AGENT_WORKSPACE_GPG_KEY` | (empty) | Default key for `auth gpg` |
| `CODEX_SECRET_DIR` | (empty) | Codex profile directory (compat) |
| `CODEX_AUTH_FILE` | `~/.codex/auth.json` | Codex auth file path (compat) |
