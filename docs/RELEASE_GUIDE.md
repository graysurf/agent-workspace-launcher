# Release Guide (agent-workspace-launcher)

Release source-of-truth is a semver tag (`vX.Y.Z`) created from `main`.

Primary release output is CLI archives consumed by Homebrew and manual installs:

- canonical executable: `agent-workspace-launcher`
- compatibility alias: `awl` (symlink to the same binary)

## Release contract

- Trigger event: push tag `vX.Y.Z` to `origin`.
- Publishing workflow: `.github/workflows/release-brew.yml`.
- Archive naming:
  - `agent-workspace-launcher-vX.Y.Z-<target>.tar.gz`
  - `agent-workspace-launcher-vX.Y.Z-<target>.tar.gz.sha256`

Supported targets:

- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`

Archive payload contract (minimum):

- `bin/agent-workspace-launcher`
- `bin/awl` (symlink to `agent-workspace-launcher`)
- `scripts/awl.bash`
- `scripts/awl.zsh`
- `completions/agent-workspace-launcher.bash`
- `completions/_agent-workspace-launcher`
- `README.md`
- `LICENSE`

## Preconditions

- Run at repo root.
- Clean worktree: `git status -sb`
- Branch is `main`: `git branch --show-current`
- GitHub CLI auth ready: `gh auth status`

## Steps

1. Decide release metadata
   - Version: `vX.Y.Z`
   - Date: `YYYY-MM-DD` (default `date +%Y-%m-%d`)

2. Run required local checks (`DEVELOPMENT.md`)
   - `bash -n $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
   - `zsh -n $(git ls-files 'scripts/*.zsh')`
   - `shellcheck $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
   - `.venv/bin/python -m ruff format --check .`
   - `.venv/bin/python -m ruff check .`
   - `.venv/bin/python -m pytest -m script_smoke`
   - `cargo fmt --all -- --check`
   - `cargo check --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test -p agent-workspace`

3. Quick binary smoke (both names map to same binary)

```sh
cargo build --release -p agent-workspace --bin agent-workspace-launcher
./target/release/agent-workspace-launcher --help

tmp_dir="$(mktemp -d)"
ln -sf "$(pwd)/target/release/agent-workspace-launcher" "$tmp_dir/awl"
"$tmp_dir/awl" --help
```

4. Update `CHANGELOG.md`
   - `./scripts/release_prepare_changelog.sh --version vX.Y.Z --date YYYY-MM-DD`

5. Commit release notes
   - Suggested message: `chore(release): vX.Y.Z`
   - Use semantic commit helper (per `AGENTS.md`).

6. Audit
   - `./scripts/release_audit.sh --version vX.Y.Z --branch main --strict`

7. Tag + push
   - `git -c tag.gpgSign=false tag vX.Y.Z`
   - `git push origin vX.Y.Z`

8. Verify release workflow + assets
   - `gh run list --workflow release-brew.yml --limit 5`
   - `gh release view vX.Y.Z --json assets --jq '.assets[].name'`

9. Download and verify checksums

```sh
version="vX.Y.Z"
out_dir="${AGENTS_HOME:-$HOME/.agents}/out/release-${version}"
mkdir -p "$out_dir"

gh release download "$version" \
  --pattern "agent-workspace-launcher-${version}-*.tar.gz" \
  --pattern "agent-workspace-launcher-${version}-*.tar.gz.sha256" \
  --dir "$out_dir"

(
  cd "$out_dir"
  for sum in *.sha256; do
    shasum -a 256 -c "$sum"
  done
)
```

10. Validate archive payload contract

```sh
version="vX.Y.Z"
archive="${AGENTS_HOME:-$HOME/.agents}/out/release-${version}/agent-workspace-launcher-${version}-x86_64-apple-darwin.tar.gz"

tar -tzf "$archive" | rg '^agent-workspace-launcher-.*-x86_64-apple-darwin/bin/(agent-workspace-launcher|awl)$'
tar -tzf "$archive" | rg '^agent-workspace-launcher-.*-x86_64-apple-darwin/completions/(agent-workspace-launcher\.bash|_agent-workspace-launcher)$'
```

## Release-to-tap checklist

1. Update `~/Project/graysurf/homebrew-tap/Formula/agent-workspace-launcher.rb` URLs + checksums from release assets.
2. Formula install contract:
   - install `bin/agent-workspace-launcher`
   - create `bin/awl` alias to same executable
   - install bash completion: `completions/agent-workspace-launcher.bash`
   - install zsh completion: `completions/_agent-workspace-launcher`
3. Validate in tap repo:
   - `ruby -c Formula/agent-workspace-launcher.rb`
   - `HOMEBREW_NO_AUTO_UPDATE=1 brew style Formula/agent-workspace-launcher.rb`
   - `brew tap graysurf/tap "$(pwd)" --custom-remote`
   - `brew update-reset "$(brew --repo graysurf/tap)"`
   - `HOMEBREW_NO_AUTO_UPDATE=1 brew upgrade graysurf/tap/agent-workspace-launcher || HOMEBREW_NO_AUTO_UPDATE=1 brew install graysurf/tap/agent-workspace-launcher`
   - `HOMEBREW_NO_AUTO_UPDATE=1 brew test agent-workspace-launcher`
4. Verify local machine actually moved to target version:
   - `.agents/skills/release-homebrew/scripts/verify-brew-installed-version.sh --version vX.Y.Z --tap-repo ~/Project/graysurf/homebrew-tap`

## Optional compatibility channel

Container-image publishing remains a separate compatibility channel, and must not gate CLI archive release correctness.

Use the project skill entrypoint:

```sh
.agents/skills/release-docker-image/scripts/release-docker-image.sh --version vX.Y.Z
```

Supported env contract (can be loaded from `.env`):

- `AWL_DOCKER_RELEASE_DOCKERHUB_USERNAME`
- `AWL_DOCKER_RELEASE_DOCKERHUB_TOKEN`
- `AWL_DOCKER_RELEASE_GHCR_USERNAME`
- `AWL_DOCKER_RELEASE_GHCR_TOKEN`
- Optional overrides:
  - `AWL_DOCKER_RELEASE_DOCKERHUB_IMAGE`
  - `AWL_DOCKER_RELEASE_GHCR_IMAGE`
  - `AWL_DOCKER_RELEASE_PLATFORMS`
  - `AWL_DOCKER_RELEASE_PUSH_LATEST`
  - `AWL_DOCKER_RELEASE_PUSH_SHA`
  - `AWL_DOCKER_RELEASE_PUSH_VERSION`

Preflight without pushing:

```sh
.agents/skills/release-docker-image/scripts/release-docker-image.sh --version vX.Y.Z --dry-run
```
