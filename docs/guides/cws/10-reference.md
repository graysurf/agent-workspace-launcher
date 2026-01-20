# Reference

## Commands

| Command | Purpose |
| --- | --- |
| `cws --help` | Show help |
| `cws ls` | List workspaces |
| `cws create ...` | Create a workspace |
| `cws exec ...` | Exec into a workspace |
| `cws rm ...` | Remove workspace(s) |
| `cws reset ...` | Reset repos inside a workspace |
| `cws tunnel ...` | Start a VS Code tunnel |

## Wrapper environment variables (host-side)

| Env | Default | Purpose |
| --- | --- | --- |
| `CWS_IMAGE` | `graysurf/codex-workspace-launcher:latest` | Launcher image to run |
| `CWS_DOCKER_ARGS` | (empty) | Extra `docker run` args for the launcher container |

## Launcher environment variables

See the full table in [README.md](../../../README.md) under “Configuration (env vars)”.
