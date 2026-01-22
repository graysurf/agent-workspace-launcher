# Development Guide

## Testing

### Setup

- Create the virtual environment: `python3 -m venv .venv`
- Install dev deps: `.venv/bin/python -m pip install -r requirements-dev.txt`

### Pre-submit checks (required)

Before submitting work, these must pass:

- Format (check): `.venv/bin/python -m ruff format --check .` (fix with `.venv/bin/python -m ruff format .`)
- Lint: `.venv/bin/python -m ruff check .`
- Smoke tests (no real Docker): `.venv/bin/python -m pytest -m script_smoke`

### Run full test suite (optional)

- `.venv/bin/python -m pytest`

### Smoke tests (no real Docker)

- `.venv/bin/python -m pytest -m script_smoke`
- These tests stub `docker` via `tests/stubs/bin` and validate the `cws` wrapper output.

### E2E tests (real Docker)

Pre-flight:

- Docker must be running: `docker info >/dev/null`
- E2E is opt-in: set `CWS_E2E=1`
- E2E runs serialize via a lock under `out/tests/e2e/` to avoid concurrent Docker runs.
  - E2E is not required before submitting work; run it only when you need real Docker coverage.

#### Run a single CLI case (one example)

Example (help case):

```sh
CWS_E2E=1 \
  CWS_E2E_CASE=help \
  .venv/bin/python -m pytest -m e2e tests/e2e/test_cws_cli_cases.py
```

Notes:

- `CWS_E2E_CASE` accepts a comma-separated list (e.g. `help,ls,create_public_https`).
- `CWS_E2E_CASE` also supports the `cli:` prefix (e.g. `cli:help`). See `tests/e2e/test_cws_cli_cases.py`.

#### Run all E2E tests

Full suite (40-case CLI matrix + bash/cli/zsh flow tests):

```sh
CWS_E2E=1 CWS_E2E_FULL=1 .venv/bin/python -m pytest -m e2e
```

#### Scope selection (feature gates)

Some cases are gated behind opt-in flags. If a gate is off (or required input is missing), pytest will **skip** the
dependent cases. Enable only what you want to exercise, or turn everything on for a “no-skip” full run.

Examples:

- Full run (safe; excludes `rm_all_yes`): set `CWS_E2E_FULL=1` and enable the `CWS_E2E_ENABLE_*` gates you want, plus
  provide the required inputs (`CWS_E2E_PUBLIC_REPO`, `CWS_E2E_PRIVATE_REPO`, `CWS_E2E_GH_TOKEN`, `CWS_E2E_CODEX_PROFILE`,
  `CWS_E2E_GPG_KEY_ID`, and (usually) `CWS_E2E_USE_HOST_HOME=1` + `CWS_DOCKER_ARGS` mounts).
- Include destructive coverage: set `CWS_E2E_ALLOW_RM_ALL=1` (includes `rm_all_yes`).
- Skip auth-heavy coverage: keep `CWS_E2E_ENABLE_AUTH=0` (also implies codex/gpg auth cases are skipped).
- Skip destructive coverage: keep `CWS_E2E_ALLOW_RM_ALL=0` (excludes `rm_all_yes` by default).

See the env table below and `tests/e2e/plan.py` for the exact skip logic.

#### E2E environment variables

Tip: you can put these into a local `.env` (gitignored) and load them via `direnv`/`.envrc`.

| Env | Required? | Purpose / Affects | Notes / Example |
| --- | --- | --- | --- |
| `CWS_E2E` | yes | Enables real Docker e2e | `1` / `true` / `yes` / `on` |
| `CWS_E2E_FULL` | optional | Runs full CLI plan suite (40 cases) and expands non-CLI flows | `1` to run everything |
| `CWS_E2E_CASE` | optional | Select specific CLI plan cases | `help` or `help,ls` |
| `CWS_E2E_IMAGE` | optional | Override launcher image tag for e2e (passed to the wrapper as `CWS_IMAGE`) | `cws-launcher:e2e` |
| `CWS_E2E_PUBLIC_REPO` | required for repo-backed cases | Enables public repo create/reset/exec cases | `OWNER/REPO` |
| `CWS_E2E_PRIVATE_REPO` | required for private repo cases | Enables `create_seed_private_repo` + `reset_private_repo` | `OWNER/PRIVATE_REPO` |
| `CWS_E2E_GH_TOKEN` | recommended (deterministic) | Token input for GitHub auth/private repo; E2E maps this to `GH_TOKEN` for the subprocess | Keep it separate from your “real” `GH_TOKEN` |
| `GH_TOKEN` / `GITHUB_TOKEN` | optional | Alternative token inputs (used by the wrapper) | If unset, the wrapper may fall back to `gh auth token`; if set, they take precedence over `CWS_E2E_GH_TOKEN` mapping |
| `CWS_E2E_ENABLE_AUTH` | optional gate | Enables `auth_*` cases | `1` to enable |
| `CWS_E2E_ENABLE_CODEX` | optional gate | Enables `auth_codex_profile` | Requires `CWS_E2E_CODEX_PROFILE` |
| `CWS_E2E_CODEX_PROFILE` | required for codex auth | Codex secret profile name | e.g. `work` (loads `~/.config/codex_secrets/work.json`) |
| `CWS_E2E_ENABLE_GPG` | optional gate | Enables `auth_gpg_key` | Requires `CWS_E2E_GPG_KEY_ID` |
| `CWS_E2E_GPG_KEY_ID` | required for gpg auth | GPG key id / fingerprint | e.g. output of `git config --global user.signingkey` |
| `CWS_E2E_ENABLE_SSH` | optional gate | Enables SSH create cases | Requires usable SSH credentials |
| `CWS_E2E_ENABLE_TUNNEL` | optional gate | Enables tunnel cases | `1` to enable |
| `CWS_E2E_ENABLE_EXEC_SHELL` | optional gate | Enables interactive `exec` shell case | `1` to enable |
| `CWS_E2E_ALLOW_RM_ALL` | optional gate (dangerous) | Enables `rm --all --yes` case | When unset, `rm_all_yes` is excluded from the default `CWS_E2E_FULL=1` CLI case selection; when set, it runs and destroys all workspace containers |
| `CWS_E2E_KEEP_WORKSPACES` | optional | Keep created workspaces for debugging | `1` to keep |
| `CWS_E2E_USE_HOST_HOME` | optional (advanced) | Use host HOME/XDG and preserve `CWS_DOCKER_ARGS` | Needed for Codex/GPG/SSH mounts |
| `CWS_DOCKER_ARGS` | optional (advanced) | Extra docker-run args for the launcher container | Use host paths (DooD): `-e HOME=\"$HOME\" -v \"$HOME/.config/codex_secrets:$HOME/.config/codex_secrets:ro\" ...` |

### Artifacts

- Smoke/test summaries and coverage are written to `out/tests/`.
