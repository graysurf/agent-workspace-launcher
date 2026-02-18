# AWL Contract (Dual Runtime)

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

- `AGENT_WORKSPACE_RUNTIME` (`container|host`, default `container`)
- `AWL_RUNTIME` (compat alias for runtime selection)
- `AGENT_WORKSPACE_HOME` (workspace root override)
- `AGENT_WORKSPACE_PREFIX` (workspace name prefix)
- `AGENT_WORKSPACE_AUTH` (`auto|gh|env|none`)
- `AGENT_WORKSPACE_GPG_KEY` (default GPG key)
- `AGENT_ENV_IMAGE` (container runtime default image for `create`)
- `CODEX_ENV_IMAGE` (compat image fallback for `create`)
- `CODEX_SECRET_DIR` (Codex compatibility)
- `CODEX_AUTH_FILE` (Codex compatibility)

## Runtime selection contract

Resolution precedence:

1. `--runtime <container|host>`
2. `AGENT_WORKSPACE_RUNTIME`
3. `AWL_RUNTIME`
4. default `container`

Notes:

- Canonical runtime values are `container` and `host`.
- Runtime parser also accepts compatibility synonyms (`docker` -> `container`, `native` -> `host`).
- Command names/subcommands are unchanged by runtime selection.

## Behavioral notes

- Default behavior is container-backed and requires host Docker access.
- Host runtime remains supported via explicit selection (`--runtime host` or env override).
- Container `create` image resolution is:
  1. `--image`
  2. `AGENT_ENV_IMAGE`
  3. `CODEX_ENV_IMAGE`
  4. built-in default `graysurf/agent-env:latest`
- `awl` remains alias-only; docs and release assets treat `agent-workspace-launcher` as canonical.

## Hard cutover

- `cws` command is removed.
- `CWS_*` runtime fallback is removed.
- `AWL_IMAGE` and `AWL_DOCKER_ARGS` are not part of the Rust CLI runtime contract.
