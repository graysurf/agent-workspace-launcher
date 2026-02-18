# Agent Workspace Rust Contract

## Purpose

Define the Rust `agent-workspace-launcher` contract implemented in this repository.

## Commands

- `agent-workspace-launcher auth ...`
- `agent-workspace-launcher create ...`
- `agent-workspace-launcher ls ...`
- `agent-workspace-launcher rm ...`
- `agent-workspace-launcher exec ...`
- `agent-workspace-launcher reset ...`
- `agent-workspace-launcher tunnel ...`
- Hidden internal: `agent-workspace-launcher __complete ...` (not listed in normal help output)

Alias contract:

- `awl ...` must resolve to the same parser and dispatch behavior.

## Runtime behavior

- Rust CLI is the runtime implementation for both backends.
- Global runtime selector:
  - flag: `--runtime <container|host>`
  - env: `AGENT_WORKSPACE_RUNTIME` (primary), `AWL_RUNTIME` (compat)
  - default: `container`
- Runtime resolution precedence: flag > `AGENT_WORKSPACE_RUNTIME` > `AWL_RUNTIME` > default.
- Exit codes reflect backend runtime result directly (`0` success, non-zero failure).

## Hidden completion contract

- `__complete` is an internal Rust entrypoint for shell completion adapters.
- Command shape:
  - `agent-workspace-launcher __complete --shell <bash|zsh> --cword <index> --word <arg0> --word <arg1> ...`
  - Optional output mode: `--format plain|describe` (`plain` default)
- `__complete` stays hidden from normal `--help` output.
- Global `--runtime` forwarding used by public subcommands is not auto-injected for `__complete`; the completion request owns its own context.
- Runtime-aware behavior:
  - Workspace-name completion for `auth`, `rm`, `exec`, `reset`, and `tunnel` resolves against the selected runtime backend.
  - Runtime precedence for completion matches command execution: `--runtime` > `AGENT_WORKSPACE_RUNTIME` > `AWL_RUNTIME` > default `container`.
- Output contract:
  - `--format plain`: newline-delimited candidates (empty output is valid).
  - `--format describe`: newline-delimited `candidate<TAB>description` lines when descriptions are available.
  - Invalid completion requests return non-zero with error text to stderr.
- Rollback toggle:
  - `AGENT_WORKSPACE_COMPLETION_MODE=legacy` switches shell adapters to legacy completion behavior.

Container backend contract:

- Executes workspace lifecycle operations via host Docker daemon.
- `create` image resolution:
  1. `--image`
  2. `AGENT_ENV_IMAGE`
  3. `CODEX_ENV_IMAGE`
  4. `graysurf/agent-env:latest`
- Container `create` post-bootstrap:
  - force-sync `~/.config/zsh` from remote `main` (default repo: `graysurf/zsh-kit`)
  - force-sync `~/.agents` from remote `main` (default repo: `graysurf/agent-kit`)
  - update `nils-cli` via Homebrew (default formula: `graysurf/tap/nils-cli`)
- `rm` supports `--keep-volumes` in container runtime.

Host backend contract:

- Executes workspace lifecycle operations on host filesystem.
- Workspace root resolution order:
  1. `AGENT_WORKSPACE_HOME`
  2. `XDG_STATE_HOME/agent-workspace-launcher/workspaces`
  3. `$HOME/.local/state/agent-workspace-launcher/workspaces`

## Naming policy

- Canonical binary name: `agent-workspace-launcher`.
- Compatibility alias: `awl`.
- No `cws` shim and no `CWS_*` fallback.
- Runtime selection does not change command names or subcommand surface.

## Codex compatibility exceptions

These env names are intentionally preserved:

- `CODEX_SECRET_DIR`
- `CODEX_AUTH_FILE`
