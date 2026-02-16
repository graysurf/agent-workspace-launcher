# `create`

Creates a host workspace directory and optionally clones repositories.

## Basic usage

```sh
agent-workspace-launcher create OWNER/REPO
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
