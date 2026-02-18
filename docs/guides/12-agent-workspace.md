# Without `awl` alias

Use the canonical binary directly:

```sh
agent-workspace-launcher --help
```

Runtime defaults:

- Default runtime is `container`.
- Host fallback is available via `--runtime host` (or `AGENT_WORKSPACE_RUNTIME=host`).

Examples:

```sh
agent-workspace-launcher create OWNER/REPO
agent-workspace-launcher ls
agent-workspace-launcher exec <workspace>
agent-workspace-launcher rm <workspace> --yes
```

Host fallback examples:

```sh
agent-workspace-launcher --runtime host create OWNER/REPO
agent-workspace-launcher --runtime host ls
```

`awl` is only a compatibility alias for the same command tree.
