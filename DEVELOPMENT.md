# Development Guide

## Setup

1. Create virtual env: `python3 -m venv .venv`
2. Install Python dev deps: `.venv/bin/python -m pip install -r requirements-dev.txt`
3. Ensure Rust toolchain is available (`rustup` + `stable` with `rustfmt` and `clippy`).

## Pre-submit checks (required)

Run all checks below before submitting changes:

- Shell syntax
  - bash: `bash -n $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
  - zsh: `zsh -n $(git ls-files 'scripts/*.zsh')`
  - completion adapters (bash): `bash -n completions/agent-workspace-launcher.bash scripts/awl.bash scripts/awl_docker.bash`
  - completion adapters (zsh): `zsh -n completions/_agent-workspace-launcher scripts/awl.zsh scripts/awl_docker.zsh`
- Shell lint (requires `shellcheck`):
  - `shellcheck $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
- Python format/lint/smoke:
  - `.venv/bin/python -m ruff format --check .`
  - `.venv/bin/python -m ruff check .`
  - `.venv/bin/python -m pytest -m script_smoke`
- Rust format/build/lint/test:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p agent-workspace`
- Completion contract smoke:
  - `bash -lc 'set -euo pipefail; ! cargo run -p agent-workspace -- --help | rg -q "__complete"'`
  - `cargo run -p agent-workspace -- __complete --shell bash --cword 2 --word agent-workspace-launcher --word rm --word "" >/dev/null`
  - `bash -lc 'cargo run -p agent-workspace -- __complete --shell invalid --cword 1 --word agent-workspace-launcher >/dev/null 2>&1; test $? -ne 0'`
  - `AGENT_WORKSPACE_COMPLETION_MODE=legacy .venv/bin/python -m pytest tests/test_completion_adapters.py -k legacy`

## Optional test commands

- Full Python suite: `.venv/bin/python -m pytest`
- Rust-only quick loop:
  - `cargo check --workspace`
  - `cargo test -p agent-workspace`

## Optional integration smoke (host-native)

Integration smoke is optional and not required for every change.

```sh
cargo build --release -p agent-workspace --bin agent-workspace-launcher

tmp_home="$(mktemp -d)"
export AGENT_WORKSPACE_HOME="${tmp_home}/workspaces"

./target/release/agent-workspace-launcher create --no-work-repos --name ws-dev-smoke
./target/release/agent-workspace-launcher ls
./target/release/agent-workspace-launcher rm ws-dev-smoke --yes
```

Alias parity smoke:

```sh
tmp_dir="$(mktemp -d)"
ln -sf "$(pwd)/target/release/agent-workspace-launcher" "${tmp_dir}/awl"
"${tmp_dir}/awl" --help
```

Legacy Docker-based E2E remains a compatibility-only path and is not part of the primary runtime gate.

## Artifacts

- Test summaries and coverage files are written under `out/tests/`.
