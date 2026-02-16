# `reset`

Resets git repos on host inside a workspace.

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
