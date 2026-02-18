# agent-workspace-launcher

Workspace lifecycle CLI for repository-focused development with dual runtimes.

- Primary command: `agent-workspace-launcher`
- Compatibility alias: `awl` (via shell wrapper or symlink)
- Runtimes: `container` (default) and `host`
- Subcommands: `auth`, `create`, `ls`, `rm`, `exec`, `reset`, `tunnel`

## Requirements

- `git` (required)
- `docker` (required for default `container` runtime)
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

If Docker is unavailable, use the `host` runtime:

```sh
agent-workspace-launcher --runtime host create OWNER/REPO
```

For Docker/source install options and full setup details, see
[Installation Guide](docs/guides/01-install.md).

## Runtime selection

- Flag: `--runtime container|host`
- Env: `AGENT_WORKSPACE_RUNTIME=container|host`
- Precedence: `--runtime` overrides `AGENT_WORKSPACE_RUNTIME`
- Default (no override): `container`

## Workspace storage

Default root:

- `AGENT_WORKSPACE_HOME` (if set)
- else `XDG_STATE_HOME/agent-workspace-launcher/workspaces`
- else `$HOME/.local/state/agent-workspace-launcher/workspaces`

## Command notes

- `create`: creates a workspace in the selected runtime and optionally clones repo(s).
- `exec`: runs a command or shell in the selected runtime workspace.
- `reset`: git reset flows (`repo`, `work-repos`, `opt-repos`, `private-repo`) in the selected runtime.
- `auth github`: stores resolved token under workspace auth directory.
- `auth codex`: syncs Codex auth files while keeping compatibility names.
- `tunnel`: runs `code tunnel` in the selected runtime workspace.

## Environment variables

| Env | Default | Purpose |
| --- | --- | --- |
| `AGENT_WORKSPACE_RUNTIME` | `container` | Runtime backend: `container\|host` |
| `AGENT_WORKSPACE_HOME` | auto | Workspace root override |
| `AGENT_WORKSPACE_PREFIX` | `agent-ws` | Prefix normalization for workspace names |
| `AGENT_WORKSPACE_AUTH` | `auto` | GitHub auth token policy: `auto\|gh\|env\|none` |
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
