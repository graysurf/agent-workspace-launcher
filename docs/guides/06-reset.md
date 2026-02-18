# `reset`

Resets git repos in a workspace for the selected runtime.

Runtime defaults to `container`. Use `--runtime host` (or
`AGENT_WORKSPACE_RUNTIME=host`) when Docker is unavailable.

## Reset one repo path

```sh
agent-workspace-launcher reset repo <workspace> /work/OWNER/REPO --yes
```

## Reset all repos under work root

```sh
agent-workspace-launcher reset work-repos <workspace> --yes
```

## Reset repos under workspace `opt/`

```sh
agent-workspace-launcher reset opt-repos <workspace> --yes
```

## Reset private repo

```sh
agent-workspace-launcher reset private-repo <workspace> --yes
```

Host runtime example:

```sh
agent-workspace-launcher --runtime host reset work-repos <workspace> --yes
```
