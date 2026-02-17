# `exec`

Runs commands (or a login shell) from a workspace path.

## Interactive shell

```sh
agent-workspace-launcher exec <workspace>
```

## Run a command

```sh
agent-workspace-launcher exec <workspace> git status
```

## Compatibility flags

`--root` / `--user` are accepted for compatibility, but ignored in host-native mode.
