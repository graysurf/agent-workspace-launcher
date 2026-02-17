# Host runtime rules

This CLI now runs host-native workspace lifecycle logic.

## Rule 1: workspace state is host filesystem state

Workspace creation/removal/reset acts on host directories under the resolved workspace root.

## Rule 2: auth artifacts are workspace-local files

`auth github/codex/gpg` writes auth metadata under workspace paths.

## Rule 3: keep compatibility env names where documented

Codex compatibility names remain:

- `CODEX_SECRET_DIR`
- `CODEX_AUTH_FILE`
