# Troubleshooting

## `agent-workspace-launcher` not found

Ensure binary is on `PATH`:

```sh
command -v agent-workspace-launcher
```

## `awl` not found

Create alias symlink:

```sh
ln -sf "$(command -v agent-workspace-launcher)" "$HOME/.local/bin/awl"
```

## Docker not available (default runtime failure)

Container is the default runtime. If Docker is unavailable, retry with host runtime:

```sh
agent-workspace-launcher --runtime host ls
```

Or override runtime for the current shell:

```sh
export AGENT_WORKSPACE_RUNTIME=host
agent-workspace-launcher ls
```

Compatibility alias (same behavior):

```sh
export AWL_RUNTIME=host
agent-workspace-launcher ls
```

## Invalid runtime value

Use `container` or `host` only:

```sh
agent-workspace-launcher --runtime container ls
agent-workspace-launcher --runtime host ls
```

## Workspace not found

Workspace state is runtime-scoped. Check both runtimes:

```sh
agent-workspace-launcher --runtime container ls
agent-workspace-launcher --runtime host ls
```

Or list with your current default:

```sh
agent-workspace-launcher ls
```

## GitHub auth issues

Use env token or host gh login:

```sh
export GH_TOKEN=...
agent-workspace-launcher auth github <workspace>
```

or

```sh
gh auth login
agent-workspace-launcher auth github <workspace>
```

## Codex auth sync issues

Check compatibility paths:

```sh
echo "$CODEX_SECRET_DIR"
echo "$CODEX_AUTH_FILE"
```

## Wrong image on `create` (container runtime)

Container `create` image precedence:

1. `--image`
2. `AGENT_ENV_IMAGE`
3. `CODEX_ENV_IMAGE`
4. `graysurf/agent-env:latest`

Pin image explicitly:

```sh
agent-workspace-launcher create --runtime container --image graysurf/agent-env:latest --no-work-repos --name ws-demo
```
