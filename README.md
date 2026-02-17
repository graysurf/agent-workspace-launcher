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

## Install

Homebrew (recommended):

```sh
brew tap graysurf/tap
brew install agent-workspace-launcher
agent-workspace-launcher --help
awl --help
```

Build from source (contributors):

```sh
cargo build --release -p agent-workspace
./target/release/agent-workspace-launcher --help
```

Create `awl` alias (optional):

```sh
ln -sf "$(pwd)/target/release/agent-workspace-launcher" "$HOME/.local/bin/awl"
awl --help
```

Docker Hub (no brew required):

```sh
docker pull graysurf/agent-workspace-launcher:latest
docker pull graysurf/agent-env:latest
```

```sh
awl_docker() {
  mkdir -p "$HOME/.awl-docker/home" "$HOME/.awl-docker/xdg-state"
  docker run --rm -it \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v "$HOME/.awl-docker:/state" \
    -e HOME=/state/home \
    -e XDG_STATE_HOME=/state/xdg-state \
    -e AGENT_ENV_IMAGE=graysurf/agent-env:latest \
    graysurf/agent-workspace-launcher:latest "$@"
}
```

```sh
awl_docker --help
awl_docker create --no-work-repos --name ws-demo
awl_docker ls
awl_docker exec ws-demo
awl_docker rm ws-demo --yes
```

## Quickstart

Create and use a workspace:

```sh
agent-workspace-launcher create OWNER/REPO
agent-workspace-launcher ls
agent-workspace-launcher exec <workspace>
agent-workspace-launcher rm <workspace> --yes
```

## `docker exec` alias and completion

Use an alias when you always target the same workspace container:

```sh
alias awx='docker exec -it agent-ws-ws-demo'
awx zsh
awx id -u
```

Use a function when container name should stay dynamic:

```sh
awxg() { docker exec -it "$@"; }
awxg agent-ws-ws-demo zsh
```

Bash completion for `awxg` first argument (container name):

```sh
_awxg_complete() {
  local cur="${COMP_WORDS[COMP_CWORD]}"
  if [[ "${COMP_CWORD}" -eq 1 ]]; then
    COMPREPLY=($(compgen -W "$(docker ps --format '{{.Names}}')" -- "${cur}"))
  fi
}
complete -F _awxg_complete awxg
```

Zsh completion for `awxg` first argument (container name):

```sh
_awxg_complete() {
  local -a names
  names=(${(f)"$(docker ps --format '{{.Names}}')"})
  _arguments "1:container:(${names[*]})" "*::cmd:_command_names -e"
}
compdef _awxg_complete awxg
```

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

These wrappers call `agent-workspace-launcher` directly and expose `aw*` shortcuts.

## Development

- Build/test guide: `docs/BUILD.md`
- Architecture: `docs/DESIGN.md`
- User guide: `docs/guides/awl/README.md`
- Release guide: `docs/RELEASE_GUIDE.md`

## License

MIT. See `LICENSE`.
