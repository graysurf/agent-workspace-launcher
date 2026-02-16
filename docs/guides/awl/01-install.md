# Install `agent-workspace-launcher` and `awl` alias

## Option A: build from source

```sh
git clone https://github.com/graysurf/agent-workspace-launcher.git
cd agent-workspace-launcher
cargo build --release -p agent-workspace

install -m 0755 target/release/agent-workspace-launcher "$HOME/.local/bin/agent-workspace-launcher"
ln -sf "$HOME/.local/bin/agent-workspace-launcher" "$HOME/.local/bin/awl"
```

## Option B: use shell wrapper alias

zsh:

```sh
source ./scripts/awl.zsh
awl --help
```

bash:

```sh
source ./scripts/awl.bash
awl --help
```

## Verify

```sh
agent-workspace-launcher --help
awl --help
```

## Notes

- `awl` is alias compatibility only.
- `agent-workspace-launcher` is the canonical command name.
