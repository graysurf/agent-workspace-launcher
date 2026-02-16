# Host AWL Contract

## Command identity

Primary command:

- `agent-workspace-launcher <subcommand> [args...]`

Alias compatibility command:

- `awl <subcommand> [args...]`

Both names must execute the same Rust implementation and behavior.

## Subcommand surface

- `auth`
- `create`
- `ls`
- `rm`
- `exec`
- `reset`
- `tunnel`

## Shell shorthand aliases

- `aw` -> `awl`
- `awa` -> `awl auth`
- `awac` -> `awl auth codex`
- `awah` -> `awl auth github`
- `awag` -> `awl auth gpg`
- `awc` -> `awl create`
- `awls` -> `awl ls`
- `awe` -> `awl exec`
- `awr` -> `awl reset`
- `awrr` -> `awl reset repo`
- `awrw` -> `awl reset work-repos`
- `awro` -> `awl reset opt-repos`
- `awrp` -> `awl reset private-repo`
- `awm` -> `awl rm`
- `awt` -> `awl tunnel`

## Runtime env contract

- `AGENT_WORKSPACE_HOME` (workspace root override)
- `AGENT_WORKSPACE_PREFIX` (workspace name prefix)
- `AGENT_WORKSPACE_AUTH` (`auto|gh|env|none`)
- `AGENT_WORKSPACE_GPG_KEY` (default GPG key)
- `CODEX_SECRET_DIR` (Codex compatibility)
- `CODEX_AUTH_FILE` (Codex compatibility)

## Behavioral notes

- Runtime must not require `docker run` or launcher-image pulls.
- Workspace lifecycle is host-directory based.
- `awl` remains alias-only; docs and release assets treat `agent-workspace-launcher` as canonical.

## Hard cutover

- `cws` command is removed.
- `CWS_*` runtime fallback is removed.
- `AWL_IMAGE` and `AWL_DOCKER_ARGS` are not part of primary runtime contract.
