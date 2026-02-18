# `tunnel`

Starts VS Code tunnel from a workspace.

Runtime defaults to `container`. Use `--runtime host` (or
`AGENT_WORKSPACE_RUNTIME=host`) when Docker is unavailable.

## Start

```sh
agent-workspace-launcher tunnel <workspace>
```

## Detach

```sh
agent-workspace-launcher tunnel <workspace> --detach
```

## Named tunnel

```sh
agent-workspace-launcher tunnel <workspace> --name <tunnel_name>
```

Host runtime example:

```sh
agent-workspace-launcher --runtime host tunnel <workspace> --detach
```
