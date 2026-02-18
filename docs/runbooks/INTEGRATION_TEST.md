# Integration test checklist (dual runtime CLI)

This checklist validates released `agent-workspace-launcher` behavior after cutting a tag (`vX.Y.Z`) across both supported runtimes:

- default `container` runtime
- explicit `host` fallback runtime

## What to verify

- [ ] `release-brew.yml` succeeded for `vX.Y.Z`
- [ ] GitHub Release has all target archives + checksums
- [ ] Archive payload contains both command names (`agent-workspace-launcher`, `awl`)
- [ ] Archive payload contains completion files for bash/zsh
- [ ] Local install smoke passes with direct binary name
- [ ] Local install smoke passes with `awl` alias
- [ ] Default runtime smoke (`container`) passes on Docker-enabled host
- [ ] Host fallback smoke (`--runtime host`) passes without Docker dependency
- [ ] Homebrew formula installs commands and completion files

## Release asset verification

```sh
version="vX.Y.Z"
out_dir="${AGENTS_HOME:-$HOME/.agents}/out/release-${version}"
mkdir -p "$out_dir"

gh run list --workflow release-brew.yml --limit 5
gh release view "$version" --json assets --jq '.assets[].name'

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

## Payload contract verification

```sh
version="vX.Y.Z"
out_dir="${AGENTS_HOME:-$HOME/.agents}/out/release-${version}"
archive="${out_dir}/agent-workspace-launcher-${version}-x86_64-apple-darwin.tar.gz"

tar -tzf "$archive" | rg '^agent-workspace-launcher-.*-x86_64-apple-darwin/bin/(agent-workspace-launcher|awl)$'
tar -tzf "$archive" | rg '^agent-workspace-launcher-.*-x86_64-apple-darwin/completions/(agent-workspace-launcher\.bash|_agent-workspace-launcher)$'
```

## Local smoke from downloaded archive

```sh
version="vX.Y.Z"
out_dir="${AGENTS_HOME:-$HOME/.agents}/out/release-${version}"
target=""
case "$(uname -s):$(uname -m)" in
  Darwin:x86_64) target="x86_64-apple-darwin" ;;
  Darwin:arm64) target="aarch64-apple-darwin" ;;
  Linux:x86_64) target="x86_64-unknown-linux-gnu" ;;
  Linux:aarch64) target="aarch64-unknown-linux-gnu" ;;
  *) echo "unsupported host target" >&2; exit 1 ;;
esac
archive="${out_dir}/agent-workspace-launcher-${version}-${target}.tar.gz"

work_dir="$(mktemp -d)"
tar -xzf "$archive" -C "$work_dir"
root_dir="$(find "$work_dir" -maxdepth 1 -type d -name "agent-workspace-launcher-${version}-*" | head -n 1)"

"$root_dir/bin/agent-workspace-launcher" --help
"$root_dir/bin/awl" --help
"$root_dir/bin/agent-workspace-launcher" --help | rg -- '--runtime'
```

## Default container runtime smoke

```sh
# Assumes root_dir is set from "Local smoke from downloaded archive".

# Requires Docker for default runtime
docker info >/dev/null

# Optional: pin runtime image explicitly for deterministic smoke
export AGENT_ENV_IMAGE="graysurf/agent-env:latest"

"$root_dir/bin/agent-workspace-launcher" create --no-work-repos --name ws-container-smoke
"$root_dir/bin/agent-workspace-launcher" ls
"$root_dir/bin/awl" ls
"$root_dir/bin/agent-workspace-launcher" exec ws-container-smoke pwd
"$root_dir/bin/agent-workspace-launcher" rm ws-container-smoke --yes
```

## Host fallback smoke

```sh
tmp_home="$(mktemp -d)"
export AGENT_WORKSPACE_HOME="$tmp_home/workspaces"

"$root_dir/bin/agent-workspace-launcher" --runtime host create --no-work-repos --name ws-host-smoke
"$root_dir/bin/agent-workspace-launcher" --runtime host ls
AGENT_WORKSPACE_RUNTIME=host "$root_dir/bin/agent-workspace-launcher" ls
AWL_RUNTIME=host "$root_dir/bin/awl" ls
"$root_dir/bin/agent-workspace-launcher" --runtime host rm ws-host-smoke --yes
```

## Homebrew tap smoke

In `~/Project/graysurf/homebrew-tap` after formula update:

```sh
ruby -c Formula/agent-workspace-launcher.rb
HOMEBREW_NO_AUTO_UPDATE=1 brew style Formula/agent-workspace-launcher.rb
brew tap graysurf/tap "$(pwd)" --custom-remote
brew update-reset "$(brew --repo graysurf/tap)"
HOMEBREW_NO_AUTO_UPDATE=1 brew upgrade graysurf/tap/agent-workspace-launcher || HOMEBREW_NO_AUTO_UPDATE=1 brew install graysurf/tap/agent-workspace-launcher
HOMEBREW_NO_AUTO_UPDATE=1 brew test agent-workspace-launcher

.agents/skills/release-homebrew/scripts/verify-brew-installed-version.sh --version vX.Y.Z --tap-repo "$(pwd)"
```

## Notes

- Release readiness requires both runtime checks: default container and host fallback.
- `awl_docker` wrapper validation is optional compatibility coverage and is not part of the required release gate.
