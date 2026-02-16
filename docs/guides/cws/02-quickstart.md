# Quickstart

Goal: create a workspace container, run commands inside it, then clean up.

## 1) Verify Docker is running

```sh
docker info >/dev/null
```

## 2) Check `cws` is working

```sh
cws --help
```

## 3) Create a workspace

```sh
cws create graysurf/agent-kit
```

The output includes:

- `workspace: <container>`
- `path: /work/<owner>/<repo>`

## 4) Run commands in the workspace

```sh
cws exec <name|container> git -C /work/graysurf/agent-kit status
```

Interactive shell:

```sh
cws exec <name|container>
```

## 5) Remove the workspace when done

```sh
cws rm <name|container> --yes
```

List workspaces:

```sh
cws ls
```
