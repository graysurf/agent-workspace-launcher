# Version bumps (upstream pins)

This repo uses a pinned upstream pair in `VERSIONS.env`:

- `ZSH_KIT_REF`: used to regenerate the bundled `bin/codex-workspace` (zsh-kit `codex-workspace` feature)
- `CODEX_KIT_REF`: used at image build time (vendored `/opt/codex-kit`, which provides the low-level launcher)

Goal: bumps are **reviewable**, **reproducible**, and validated with this repo’s real-Docker E2E suite.

## When to bump

- You merged a contract change in `zsh-kit` and/or `codex-kit` that must ship in the launcher image.
- You need to roll forward/back to fix a regression or pin a known-good pair.

## Automated bump (recommended)

Use the helper script to update pins, regenerate the bundle, run checks, build, and verify the image:

```sh
./scripts/bump_versions.sh --from-main
```

To also run real-Docker E2E (full matrix) against the built image:

```sh
./scripts/bump_versions.sh --from-main --run-e2e
```

Notes:

- `--from-main` keeps backward-compatible CLI naming, but resolves `zsh-kit` from `refs/heads/nils-cli` and `codex-kit` from `refs/heads/main`.
- The bundle regeneration uses the pinned `zsh-kit` tool (`tools/bundle-wrapper.zsh`) at `ZSH_KIT_REF`.
- You do not need `~/.config/zsh` on your machine unless you want a local fallback.
- E2E requires your local env to be configured (see “Real-Docker E2E” below for required env vars).
- Destructive coverage (`rm --all --yes`) is still gated by `CWS_E2E_ALLOW_RM_ALL=1` (use with care).

Pin explicitly (still resolves to full commit SHAs and writes them into `VERSIONS.env`):

```sh
./scripts/bump_versions.sh \
  --zsh-kit-ref <ref|sha> \
  --codex-kit-ref <ref|sha>
```

Tip: use `--skip-docker` when you only want to update files + run tests locally.

## Choose new pins

Recommended: pin by commit SHA (most deterministic).

Example: pin to upstream default release heads (replace URLs/refs as needed):

```sh
git ls-remote https://github.com/graysurf/zsh-kit.git refs/heads/nils-cli | awk '{print $1}' | head -n 1
git ls-remote https://github.com/graysurf/codex-kit.git refs/heads/main | awk '{print $1}' | head -n 1
```

Tip: `zsh-kit` and `codex-kit` pins must be compatible as a pair (wrapper expects launcher capabilities).

## Update `VERSIONS.env`

Edit `VERSIONS.env` at repo root:

- `ZSH_KIT_REF=<sha>`
- `CODEX_KIT_REF=<sha>`

## Regenerate the bundled wrapper (required)

When `ZSH_KIT_REF` changes, regenerate and commit the bundled wrapper:

```sh
./scripts/generate_codex_workspace_bundle.sh
```

Sanity check:

```sh
head -n 5 ./bin/codex-workspace
```

## Build a local image using the pinned refs

```sh
set -euo pipefail
set -a
source ./VERSIONS.env
set +a

docker build -t codex-workspace-launcher:local \
  --build-arg ZSH_KIT_REF="$ZSH_KIT_REF" \
  --build-arg CODEX_KIT_REF="$CODEX_KIT_REF" \
  .
```

## Verify the built image contains the pins

```sh
docker run --rm --entrypoint cat codex-workspace-launcher:local /opt/zsh-kit.ref
docker run --rm --entrypoint cat codex-workspace-launcher:local /opt/codex-kit/.ref
```

You can also inspect labels:

```sh
docker inspect codex-workspace-launcher:local --format '{{json .Config.Labels}}' | jq .
```

## Required repo checks (before opening a PR)

See `DEVELOPMENT.md`. At minimum:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-dev.txt

.venv/bin/python -m ruff format --check .
.venv/bin/python -m ruff check .
.venv/bin/python -m pytest -m script_smoke
```

## Real-Docker E2E (launcher image validation)

E2E is opt-in (real Docker). Keep it here (launcher repo) and keep upstream repos on smoke/stub tests.

Minimal example (help case):

```sh
CWS_E2E=1 \
  CWS_E2E_IMAGE=codex-workspace-launcher:local \
  CWS_E2E_CASE=help \
  .venv/bin/python -m pytest -m e2e tests/e2e/test_cws_cli_cases.py
```

Recommended: run the wrapper flow tests (real Docker; creates workspaces and cleans them up):

```sh
CWS_E2E=1 \
  CWS_AUTH=none \
  CWS_E2E_IMAGE=codex-workspace-launcher:local \
  CWS_E2E_PUBLIC_REPO=graysurf/codex-kit \
  .venv/bin/python -m pytest -m e2e \
    tests/e2e/test_cws_cli_plan.py \
    tests/e2e/test_cws_bash_plan.py \
    tests/e2e/test_cws_zsh_plan.py
```

Artifacts are written under `out/tests/e2e/`.

Notes:

- Full matrix: `CWS_E2E=1 CWS_E2E_FULL=1 .venv/bin/python -m pytest -m e2e`
- Destructive coverage (`rm --all --yes`) is gated by `CWS_E2E_ALLOW_RM_ALL=1` (use with care).

## Publish

This repo’s publish workflow triggers on pushes to the `docker` branch (see `.github/workflows/publish.yml`).

Recommended pattern:

1. Land changes on `main` via PR.
2. Fast-forward `docker` to `main` when you want to publish.
3. Verify tags (`latest`, `sha-<short>`) exist in Docker Hub / GHCR.
