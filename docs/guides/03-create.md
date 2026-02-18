# `create`

Creates a workspace in the selected runtime and optionally clones repositories.

Runtime selection:

- Default: `container`
- Override per command: `--runtime container|host`
- Override via env: `AGENT_WORKSPACE_RUNTIME=container|host`

## Basic usage

```sh
agent-workspace-launcher create OWNER/REPO
```

Host fallback (for Docker-unavailable hosts):

```sh
agent-workspace-launcher --runtime host create OWNER/REPO
```

## Multiple repos

```sh
agent-workspace-launcher create OWNER/REPO OTHER/REPO
```

## Explicit workspace name

```sh
agent-workspace-launcher create --name ws-foo OWNER/REPO
```

## Empty workspace

```sh
agent-workspace-launcher create --no-work-repos --name ws-empty
```

## Seed private repo directory

```sh
agent-workspace-launcher create --private-repo OWNER/PRIVATE_REPO OWNER/REPO
```

Alias form:

```sh
awl create OWNER/REPO
```
