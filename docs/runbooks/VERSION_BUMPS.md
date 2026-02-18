# Version bumps (upstream pin)

Primary runtime is the Rust binary in this repo with dual backends (`container` default, `host` fallback).
`AGENT_KIT_REF` remains only for optional compatibility image builds.

## Current release focus

When preparing a release, prioritize:

- binary/runtime checks in this repo
- CLI archive packaging (`release-brew.yml`)
- name contract: `agent-workspace-launcher` primary, `awl` alias

## When to bump `AGENT_KIT_REF`

Bump only if you intentionally update compatibility container assets that still vendor `agent-kit`.
If you are shipping CLI-only changes, you can skip container pin updates.

## Update pin (optional compatibility path)

```sh
./scripts/bump_versions.sh --from-main --skip-docker
```

Pin explicitly:

```sh
./scripts/bump_versions.sh --agent-kit-ref <ref|sha> --skip-docker
```

This updates `VERSIONS.env` and runs repo checks without requiring Docker.

## Required checks before PR/release

Run all checks from `DEVELOPMENT.md`:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-dev.txt

bash -n $(git ls-files 'scripts/*.sh' 'scripts/*.bash')
zsh -n $(git ls-files 'scripts/*.zsh')
shellcheck $(git ls-files 'scripts/*.sh' 'scripts/*.bash')
.venv/bin/python -m ruff format --check .
.venv/bin/python -m ruff check .
.venv/bin/python -m pytest -m script_smoke
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p agent-workspace
```

## Publish (CLI-first)

Release source-of-truth is a semver tag (`vX.Y.Z`) from `main`.

1. Land changes on `main` via PR.
2. Create and push tag:

```sh
git -c tag.gpgSign=false tag vX.Y.Z
git push origin vX.Y.Z
```

3. Ensure release exists:

```sh
gh release view vX.Y.Z || gh release create vX.Y.Z --title "vX.Y.Z" --notes ""
```

4. Verify CLI workflow (`release-brew.yml`) succeeded and published all expected assets.

## CLI channel verification checkpoints

- `gh run list --workflow release-brew.yml --limit 5`
- `gh release view vX.Y.Z --json assets --jq '.assets[].name'`
- Download + checksum verify:
  - `mkdir -p "${AGENTS_HOME:-$HOME/.agents}/out/release-vX.Y.Z"`
  - `gh release download vX.Y.Z --pattern '*.tar.gz' --pattern '*.sha256' --dir "${AGENTS_HOME:-$HOME/.agents}/out/release-vX.Y.Z"`
  - `cd "${AGENTS_HOME:-$HOME/.agents}/out/release-vX.Y.Z" && for f in *.sha256; do shasum -a 256 -c "$f"; done`
- Verify payload includes both command names:
  - `tar -tzf agent-workspace-launcher-vX.Y.Z-x86_64-apple-darwin.tar.gz | rg 'bin/(agent-workspace-launcher|awl)$'`
- Verify payload includes completion files:
  - `tar -tzf agent-workspace-launcher-vX.Y.Z-x86_64-apple-darwin.tar.gz | rg 'completions/(agent-workspace-launcher\.bash|_agent-workspace-launcher)$'`

## Update `homebrew-tap` formula

After checks pass, update the tap formula with exact URL + checksum pairs and keep this install contract:

- installs `agent-workspace-launcher` as primary binary
- installs `awl` as alias/symlink
- installs bash completion from `completions/agent-workspace-launcher.bash`
- installs zsh completion from `completions/_agent-workspace-launcher`

Validate in `homebrew-tap`:

```sh
ruby -c Formula/agent-workspace-launcher.rb
HOMEBREW_NO_AUTO_UPDATE=1 brew style Formula/agent-workspace-launcher.rb
brew tap graysurf/tap "$(pwd)" --custom-remote
brew update-reset "$(brew --repo graysurf/tap)"
HOMEBREW_NO_AUTO_UPDATE=1 brew upgrade graysurf/tap/agent-workspace-launcher || HOMEBREW_NO_AUTO_UPDATE=1 brew install graysurf/tap/agent-workspace-launcher
HOMEBREW_NO_AUTO_UPDATE=1 brew test agent-workspace-launcher
.agents/skills/release-homebrew/scripts/verify-brew-installed-version.sh --version vX.Y.Z --tap-repo "$(pwd)"
```

## Optional compatibility Docker checks

If you also publish compatibility container artifacts, run Docker-specific verification separately. That path is optional and must not gate CLI release readiness.

```sh
.agents/skills/release-docker-image/scripts/release-docker-image.sh --version vX.Y.Z --dry-run
.agents/skills/release-docker-image/scripts/release-docker-image.sh --version vX.Y.Z
```
