# Quickstart

Goal: create a workspace, run commands inside it, then clean up.

Runtime defaults:

- `agent-workspace-launcher` supports `container` and `host`
- Default runtime is `container`
- Use `--runtime container|host` or `AGENT_WORKSPACE_RUNTIME`

## 1) Verify CLI

```sh
agent-workspace-launcher --help
```

## 2) Create a workspace

```sh
agent-workspace-launcher create --no-work-repos --name ws-demo
```

Or clone a repo during create:

```sh
agent-workspace-launcher create OWNER/REPO
```

If Docker is unavailable, switch to host runtime:

```sh
agent-workspace-launcher --runtime host create --no-work-repos --name ws-demo
```

## 3) List workspaces

```sh
agent-workspace-launcher ls
```

## 4) Run commands

```sh
agent-workspace-launcher exec ws-demo pwd
agent-workspace-launcher exec ws-demo
```

## 5) Remove workspace

```sh
agent-workspace-launcher rm ws-demo --yes
```

Optional: run the full flow in host mode via env override:

```sh
export AGENT_WORKSPACE_RUNTIME=host
agent-workspace-launcher create --no-work-repos --name ws-host
agent-workspace-launcher ls
agent-workspace-launcher exec ws-host pwd
agent-workspace-launcher rm ws-host --yes
unset AGENT_WORKSPACE_RUNTIME
```
