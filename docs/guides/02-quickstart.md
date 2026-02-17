# Quickstart

Goal: create a workspace, run commands inside it, then clean up.

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
