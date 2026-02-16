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

## Workspace not found

List known workspaces:

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
