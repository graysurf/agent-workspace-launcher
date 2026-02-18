# Guides

User-facing documentation lives here. The goal is to keep each guide task-oriented,
with copy/paste-friendly commands and small gotcha sections.

Related docs:

- Design reference: [docs/DESIGN.md](../DESIGN.md)
- Local builds: [docs/BUILD.md](../BUILD.md)
- Runbooks (developer checklists): [docs/runbooks/](../runbooks/)
- Progress tracking: [docs/progress/](../progress/)

## awl / agent-workspace-launcher guide

`agent-workspace-launcher` is the canonical CLI with dual runtime support.

`awl` is a compatibility alias that calls the same binary.

Default runtime is `container`; `host` runtime remains available via `--runtime host`.

This guide covers install, runtime selection, workspace lifecycle commands, and troubleshooting.

## Start here

1. Install: `docs/guides/01-install.md`
2. Quickstart: `docs/guides/02-quickstart.md`

## Command guides

- Create workspaces: `03-create.md`
- Exec commands/shell: `04-exec.md`
- Remove workspaces: `05-rm.md`
- Reset repos: `06-reset.md`
- VS Code tunnel: `07-tunnel.md`
- Auth updates: `08-auth.md`

## Concepts and reference

- Dual runtime rules: `09-dood-rules.md`
- Troubleshooting: `10-troubleshooting.md`
- Reference: `11-reference.md`
- Without `awl` alias: `12-agent-workspace.md`
