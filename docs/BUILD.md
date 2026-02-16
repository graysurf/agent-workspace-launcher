# Build guide

This guide builds and runs the host-native `agent-workspace-launcher` binary.

## Requirements

- Rust toolchain (`rustup`, stable)
- `git`

## Build

```sh
git clone https://github.com/graysurf/agent-workspace-launcher.git
cd agent-workspace-launcher

cargo build --release -p agent-workspace
```

Binary output:

```sh
./target/release/agent-workspace-launcher --help
```

## Optional alias (`awl`)

Use wrapper scripts:

```sh
source ./scripts/awl.bash
awl --help
```

or symlink:

```sh
ln -sf "$(pwd)/target/release/agent-workspace-launcher" "$HOME/.local/bin/awl"
awl --help
```

## Local smoke

```sh
agent-workspace-launcher create --no-work-repos --name ws-local
agent-workspace-launcher ls
agent-workspace-launcher rm ws-local --yes
```

## Tests

Run required checks from `DEVELOPMENT.md` before submission.
