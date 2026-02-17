# agent-workspace-launcher

Host-native workspace lifecycle CLI for repository-focused development.

- Primary command: `agent-workspace-launcher`
- Compatibility alias: `awl` (via shell wrapper or symlink)
- Host-native usage is the primary path; Docker image usage is optional
- Subcommands: `auth`, `create`, `ls`, `rm`, `exec`, `reset`, `tunnel`

## Requirements

- `git` (required)
- Optional for specific flows:
  - `gh` (GitHub token/keyring auth)
  - `gpg` (signing key checks)
  - `code` (VS Code tunnel)

## Quickstart

Install with Homebrew:

```sh
brew tap graysurf/tap
brew install agent-workspace-launcher
```

Create and use a workspace:

```sh
agent-workspace-launcher create OWNER/REPO
agent-workspace-launcher ls
agent-workspace-launcher exec <workspace>
agent-workspace-launcher rm <workspace> --yes
```

For Docker/source install options and full setup details, see
[Installation Guide](docs/guides/01-install.md).

## Workspace storage

Default root:

- `AGENT_WORKSPACE_HOME` (if set)
- else `XDG_STATE_HOME/agent-workspace-launcher/workspaces`
- else `$HOME/.local/state/agent-workspace-launcher/workspaces`

## Command notes

- `create`: makes a host workspace directory and optionally clones repo(s).
- `exec`: runs command or login shell from workspace path.
- `reset`: host-side git reset flows (`repo`, `work-repos`, `opt-repos`, `private-repo`).
- `auth github`: stores resolved host token under workspace auth directory.
- `auth codex`: syncs Codex auth files while keeping compatibility names.
- `tunnel`: runs `code tunnel` from workspace path.

## Environment variables

| Env | Default | Purpose |
| --- | --- | --- |
| `AGENT_WORKSPACE_HOME` | auto | Workspace root override |
| `AGENT_WORKSPACE_PREFIX` | `agent-ws` | Prefix normalization for workspace names |
| `AGENT_WORKSPACE_AUTH` | `auto` | GitHub auth token policy: `auto|gh|env|none` |
| `AGENT_WORKSPACE_GPG_KEY` | (empty) | Default key for `auth gpg` |
| `CODEX_SECRET_DIR` | (empty) | Codex profile directory (compatibility name) |
| `CODEX_AUTH_FILE` | `~/.codex/auth.json` | Codex auth file path (compatibility name) |

## Alias wrappers

- `scripts/awl.bash`
- `scripts/awl.zsh`
- `scripts/awl_docker.bash`
- `scripts/awl_docker.zsh`

These wrappers call `agent-workspace-launcher` directly and expose `aw*` shortcuts.

## Development

- Build/test guide: `docs/BUILD.md`
- Architecture: `docs/DESIGN.md`
- User guide: `docs/guides/README.md`
- Release guide: `docs/RELEASE_GUIDE.md`

## License

MIT. See `LICENSE`.
