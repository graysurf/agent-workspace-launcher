# Reference

## Commands

| Command | Purpose |
| --- | --- |
| `cws --help` | Show help |
| `cws auth ...` | Update auth inside a workspace |
| `cws ls` | List workspaces |
| `cws create ...` | Create a workspace |
| `cws exec ...` | Exec into a workspace |
| `cws rm ...` | Remove workspace(s) |
| `cws reset ...` | Reset repos inside a workspace |
| `cws tunnel ...` | Start a VS Code tunnel |

## Wrapper environment variables (host-side)

| Env | Default | Purpose |
| --- | --- | --- |
| `CWS_IMAGE` | `graysurf/agent-workspace-launcher:latest` | Launcher image to run |
| `CWS_DOCKER_ARGS` | (empty) | Extra `docker run` args for the launcher container |
| `CWS_AUTH` | `auto` | `auto\|env\|none`: when `auto`, reuse host `gh` keyring token for `create/reset/auth github` if no `GH_TOKEN`/`GITHUB_TOKEN` |

## Launcher environment variables

See the full table in [README.md](../../../README.md) under “Configuration (env vars)”.
