# Dual runtime rules

This CLI supports two runtime backends with container as the default.

## Rule 1: runtime selection is explicit and deterministic

Selection precedence:

1. `--runtime <container|host>`
2. `AGENT_WORKSPACE_RUNTIME`
3. `AWL_RUNTIME`
4. default `container`

## Rule 2: command surface stays the same

Subcommands do not change by runtime: `auth`, `create`, `ls`, `rm`, `exec`, `reset`, `tunnel`.

## Rule 3: workspace state is runtime-scoped

- `container` runtime: workspace state is Docker containers + volumes.
- `host` runtime: workspace state is host filesystem under resolved workspace root.

## Rule 4: container image selection for `create` is stable

When runtime is `container`, image precedence is:

1. `create --image <image>`
2. `AGENT_ENV_IMAGE`
3. `CODEX_ENV_IMAGE`
4. `graysurf/agent-env:latest`

## Rule 5: keep compatibility env names where documented

Codex compatibility names remain:

- `CODEX_SECRET_DIR`
- `CODEX_AUTH_FILE`
