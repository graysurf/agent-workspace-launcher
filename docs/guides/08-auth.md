# `auth`

Updates auth material for an existing workspace.

Runtime defaults to `container`. Use `--runtime host` or
`AGENT_WORKSPACE_RUNTIME=host` when Docker is unavailable.

Providers:

- `github`
- `codex`
- `gpg`

## GitHub

```sh
agent-workspace-launcher auth github <workspace>
```

Host runtime example:

```sh
agent-workspace-launcher --runtime host auth github <workspace>
```

Policy (`AGENT_WORKSPACE_AUTH`):

- `auto`: prefer `gh auth token`, fallback env
- `gh`: require `gh` keyring token (fallback warning)
- `env`: use `GH_TOKEN` / `GITHUB_TOKEN`
- `none`: disable token resolution

## Codex

```sh
agent-workspace-launcher auth codex --profile work <workspace>
```

Compatibility names are preserved:

- `CODEX_SECRET_DIR`
- `CODEX_AUTH_FILE`

## GPG

```sh
agent-workspace-launcher auth gpg --key <keyid|fingerprint> <workspace>
```
