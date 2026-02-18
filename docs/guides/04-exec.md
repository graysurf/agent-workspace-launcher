# `exec`

Runs commands (or a login shell) in a workspace.

Runtime defaults to `container`. Use `--runtime host` or
`AGENT_WORKSPACE_RUNTIME=host` when Docker is unavailable.

## Interactive shell

```sh
agent-workspace-launcher exec <workspace>
```

## Run a command

```sh
agent-workspace-launcher exec <workspace> git status
```

## Host runtime example

```sh
agent-workspace-launcher --runtime host exec <workspace> git status
```

## Compatibility flags

`--root` / `--user` are accepted for compatibility.

- In `container` runtime they select container user.
- In `host` runtime they are ignored with a warning.
