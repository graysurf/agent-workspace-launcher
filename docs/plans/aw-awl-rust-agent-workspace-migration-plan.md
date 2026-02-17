# Plan: AW/AWL Host Migration and Rust `agent-workspace` Cutover

## Overview
This plan replaces the current zsh-bundled `bin/agent-workspace` with a Rust crate CLI implemented in this repository, and migrates host-side `cws` usage to `aw*/awl*` naming (including env vars and test names). The new runtime keeps using `agent-env` from `agent-kit`, but all launcher behavior in this repo becomes Rust-owned code. We will keep `CODEX_SECRET_DIR` and `CODEX_AUTH_FILE` unchanged by design, because they are Codex-specific compatibility inputs. The rollout includes contract specs, implementation, CI/publish updates, and end-to-end validation.

## Scope
- In scope: Rust crate CLI (`agent-workspace`) replacing `bin/agent-workspace` script.
- In scope: Host command/env/test naming migration from `cws`/`CWS_*` to `awl`/`AWL_*`, and `cw*` to `aw*` shorthand.
- In scope: Remove dependency on `scripts/bundles/agent-workspace.wrapper.zsh` and `scripts/generate_agent_workspace_bundle.sh`.
- In scope: Validate integration path where `agent-workspace` drives `agent-env` behavior.
- In scope: Update Docker build, CI, publish workflow, runbooks, and user docs.
- Out of scope: Refactoring internals inside `/Users/terry/Project/graysurf/agent-kit` beyond consumption contract checks.
- Out of scope: Renaming Codex-specific env names `CODEX_SECRET_DIR` and `CODEX_AUTH_FILE`.
- Out of scope: Historical rewrite of archived progress docs; only add migration notes where needed.

## Assumptions (if any)
1. Breaking API migration to `awl`/`AWL_*` is acceptable for primary docs/tests in this repo.
2. This migration is a hard cutover: no `cws` shim and no `CWS_*` runtime fallback are kept.
3. Docker remains the execution substrate; no non-Docker runtime path is added.
4. Rust patterns should align with `/Users/terry/Project/graysurf/nils-cli` (workspace layout, clap-based parsing, test organization).

## Sprint 1: Contract Freeze and Migration Inventory
**Goal**: Freeze the command/env naming contract and migration boundary before code changes.
**Demo/Validation**:
- Command(s): `rg -n "awl|aw|agent-workspace|CODEX_SECRET_DIR|CODEX_AUTH_FILE" docs/specs docs/plans`
- Verify: Command and env mapping is explicit, and Codex env exceptions are documented.

### Task 1.1: Define new CLI and host contract spec
- **Location**:
  - `docs/specs/agent-workspace-rust-contract.md`
  - `docs/specs/host-awl-contract.md`
- **Description**: Document command surface, option semantics, output expectations, exit codes, naming migration (`cws`/`CWS_*` -> `awl`/`AWL_*`, `cw*` -> `aw*`), and explicit non-renaming of `CODEX_SECRET_DIR` + `CODEX_AUTH_FILE`.
- **Dependencies**:
  - none
- **Complexity**: 4
- **Acceptance criteria**:
  - Contract docs enumerate all subcommands (`auth/create/ls/rm/exec/reset/tunnel`) and host wrapper behavior.
  - Contract docs explicitly list Codex env exceptions unchanged.
- **Validation**:
  - `rg -n "CODEX_SECRET_DIR|CODEX_AUTH_FILE|CWS_|AWL_|cw|aw" docs/specs/agent-workspace-rust-contract.md docs/specs/host-awl-contract.md`

### Task 1.2: Build migration inventory and file touch map
- **Location**:
  - `docs/plans/aw-awl-migration-inventory.md`
- **Description**: Record affected files by category: runtime CLI, host wrappers, tests, CI workflows, release scripts, docs, and removed zsh bundle assets.
- **Dependencies**:
  - none
- **Complexity**: 3
- **Acceptance criteria**:
  - Inventory includes every current `scripts/cws.*`, `tests/e2e/test_cws_*`, `CWS_*` variable path, and zsh bundle generation path.
- **Validation**:
  - `rg -n "scripts/cws|test_cws|CWS_|generate_agent_workspace_bundle|agent-workspace.wrapper.zsh" docs/plans/aw-awl-migration-inventory.md`

### Task 1.3: Define parity test matrix vs current behavior
- **Location**:
  - `docs/specs/agent-workspace-parity-matrix.md`
- **Description**: Create a parity checklist comparing current behavior and target Rust behavior for normal cases, auth flows, tunnel flows, and destructive operations.
- **Dependencies**:
  - Task 1.1
  - Task 1.2
- **Complexity**: 5
- **Acceptance criteria**:
  - Matrix includes baseline smoke, e2e gated cases, and error-path expectations.
  - Matrix includes explicit agent-env integration checks.
- **Validation**:
  - `rg -n "parity|agent-env|auth|tunnel|rm --all" docs/specs/agent-workspace-parity-matrix.md`

## Sprint 2: Rust Workspace Bootstrap (nils-cli style)
**Goal**: Create a Rust workspace foundation and compile a minimal CLI skeleton.
**Demo/Validation**:
- Command(s): `cargo fmt --all -- --check && cargo check --workspace`
- Verify: Workspace builds cleanly and exposes a parseable CLI skeleton.

### Task 2.1: Initialize Rust workspace metadata
- **Location**:
  - `Cargo.toml`
  - `Cargo.lock`
  - `rust-toolchain.toml`
  - `rustfmt.toml`
- **Description**: Set up workspace metadata aligned with `nils-cli` conventions (edition, lint policy, shared dependencies).
- **Dependencies**:
  - Task 1.1
- **Complexity**: 4
- **Acceptance criteria**:
  - `cargo check --workspace` runs successfully.
  - Toolchain and formatting config are pinned and documented.
- **Validation**:
  - `cargo check --workspace`

### Task 2.2: Scaffold `agent-workspace` crate and entrypoints
- **Location**:
  - `crates/agent-workspace/Cargo.toml`
  - `crates/agent-workspace/src/main.rs`
  - `crates/agent-workspace/src/lib.rs`
  - `crates/agent-workspace/src/cli.rs`
- **Description**: Implement clap-based command tree and top-level routing with stable exit codes.
- **Dependencies**:
  - Task 2.1
- **Complexity**: 5
- **Acceptance criteria**:
  - `agent-workspace --help` includes target subcommands.
  - `agent-workspace` exits non-zero for invalid args with deterministic codes.
- **Validation**:
  - `cargo run -p agent-workspace -- --help`
  - `cargo run -p agent-workspace -- not-a-command; test $? -ne 0`

### Task 2.3: Add shared runtime modules for env and docker argv
- **Location**:
  - `crates/agent-workspace/src/env.rs`
  - `crates/agent-workspace/src/docker_args.rs`
  - `crates/agent-workspace/src/errors.rs`
- **Description**: Create typed parsing for `AWL_*`/compat env values, pass-through logic, and docker argv assembly utilities.
- **Dependencies**:
  - Task 2.2
- **Complexity**: 6
- **Acceptance criteria**:
  - Env parsing supports target `AWL_*` inputs only (no `CWS_*` fallback).
  - Docker argv builder is unit-testable without invoking Docker.
- **Validation**:
  - `cargo test -p agent-workspace env::tests && cargo test -p agent-workspace docker_args::tests`

## Sprint 3: Implement Rust Runtime Behavior
**Goal**: Reach functional parity for `agent-workspace` operations and host invocation paths.
**Demo/Validation**:
- Command(s): `cargo test -p agent-workspace`
- Verify: Command routing, env handling, and argument transformation match parity matrix.

### Task 3.1: Implement core command handlers (`auth/create/ls/rm/exec/reset/tunnel`)
- **Location**:
  - `crates/agent-workspace/src/commands/mod.rs`
  - `crates/agent-workspace/src/commands/auth.rs`
  - `crates/agent-workspace/src/commands/create.rs`
  - `crates/agent-workspace/src/commands/ls.rs`
  - `crates/agent-workspace/src/commands/rm.rs`
  - `crates/agent-workspace/src/commands/exec.rs`
  - `crates/agent-workspace/src/commands/reset.rs`
  - `crates/agent-workspace/src/commands/tunnel.rs`
- **Description**: Implement subcommand behavior in Rust and invoke the low-level launcher contract where required.
- **Dependencies**:
  - Task 2.3
- **Complexity**: 8
- **Acceptance criteria**:
  - Subcommands execute with expected argument semantics.
  - Help and error messages align with defined contract.
- **Validation**:
  - `cargo test -p agent-workspace commands::`

### Task 3.2: Implement host wrapper mode (`awl`) and auth token injection
- **Location**:
  - `crates/agent-workspace/src/host_wrapper.rs`
  - `crates/agent-workspace/src/github_auth.rs`
- **Description**: Implement host-side invocation flow currently owned by `cws`, including docker socket mount defaults, token forwarding, and optional gh keyring acquisition. Shell wrapper rename and script ownership transfer are handled in Task 5.1.
- **Dependencies**:
  - Task 2.3
- **Complexity**: 8
- **Acceptance criteria**:
  - `awl auth|create|ls|rm|exec|reset|tunnel` produces equivalent docker invocation behavior.
  - `AWL_AUTH=env|none|auto` behavior is deterministic.
- **Validation**:
  - `cargo test -p agent-workspace host_wrapper::tests && cargo test -p agent-workspace github_auth::tests`

### Task 3.3: Implement `aw*` shorthand and completion contract
- **Location**:
  - `crates/agent-workspace/src/shorthand.rs`
  - `completions/awl.bash`
  - `completions/_aws`
- **Description**: Replace `cw*` shorthand with `aw*` mapping and generate/update completion scripts for bash and zsh.
- **Dependencies**:
  - Task 3.2
- **Complexity**: 6
- **Acceptance criteria**:
  - `aw`, `awa`, `awc`, `awl`, `awe`, `awr`, `awm`, `awt` map correctly.
  - Completion includes new command/env naming.
- **Validation**:
  - `cargo test -p agent-workspace shorthand::tests`
  - `rg -n "\baw[a-z]*\b|\bcw[a-z]*\b" completions scripts`

### Task 3.4: Preserve Codex-specific env names unchanged
- **Location**:
  - `crates/agent-workspace/src/codex_compat.rs`
  - `crates/agent-workspace/tests/codex_env_passthrough.rs`
- **Description**: Ensure `CODEX_SECRET_DIR` and `CODEX_AUTH_FILE` remain supported names without renaming in command behavior and docs/examples.
- **Dependencies**:
  - Task 3.1
- **Complexity**: 4
- **Acceptance criteria**:
  - Both env names are consumed exactly as-is.
  - No migration logic rewrites these names to `AGENT_*` variants.
- **Validation**:
  - `cargo test -p agent-workspace codex_env_passthrough`
  - `rg -n "CODEX_SECRET_DIR|CODEX_AUTH_FILE" crates/agent-workspace`

## Sprint 4: Packaging and Pipeline Cutover (Remove zsh bundle)
**Goal**: Make image builds and CI independent of zsh bundle generation.
**Demo/Validation**:
- Command(s): `docker build -t agent-workspace-launcher:rust-local .`
- Verify: Built image entrypoint is Rust binary and runtime help works.

### Task 4.1: Replace script entrypoint with Rust binary in Dockerfile
- **Location**:
  - `Dockerfile`
  - `.dockerignore`
- **Description**: Add Rust build stage, copy compiled `agent-workspace` binary into runtime image, and remove script-copy entrypoint behavior.
- **Dependencies**:
  - Task 3.1
- **Complexity**: 7
- **Acceptance criteria**:
  - Image builds on both amd64/arm64 via buildx.
  - `docker run --rm agent-workspace-launcher:rust-local --help` succeeds.
- **Validation**:
  - `docker buildx build --platform linux/amd64,linux/arm64 -t agent-workspace-launcher:rust-local --load .`
  - `docker run --rm agent-workspace-launcher:rust-local --help`

### Task 4.2: Remove zsh bundle generator and vendored bundle artifacts
- **Location**:
  - `scripts/generate_agent_workspace_bundle.sh`
  - `scripts/bundles/agent-workspace.wrapper.zsh`
  - `bin/agent-workspace`
  - `tests/script_specs/scripts/generate_agent_workspace_bundle.sh.json`
  - `tests/script_specs/scripts/bundles/agent-workspace.wrapper.zsh.json`
- **Description**: Delete obsolete bundle chain and replace tests that reference it with Rust-binary-oriented checks.
- **Dependencies**:
  - Task 4.1
- **Complexity**: 6
- **Acceptance criteria**:
  - No build/test path references `agent-workspace.wrapper.zsh`.
  - CI no longer requires zsh syntax checks for bundled binary script.
- **Validation**:
  - `test ! -e scripts/generate_agent_workspace_bundle.sh && test ! -e scripts/bundles/agent-workspace.wrapper.zsh && test ! -e bin/agent-workspace`
  - `! rg -n "generate_agent_workspace_bundle|agent-workspace.wrapper.zsh" .github scripts tests docs README.md DEVELOPMENT.md --glob '!docs/plans/**' --glob '!docs/progress/archived/**'`

### Task 4.3: Update version pinning and release scripts for new architecture
- **Location**:
  - `VERSIONS.env`
  - `scripts/bump_versions.sh`
  - `scripts/release_prepare_changelog.sh`
  - `scripts/release_audit.sh`
  - `docs/runbooks/VERSION_BUMPS.md`
- **Description**: Remove zsh-kit pin assumptions, keep relevant upstream pins (agent-kit/agent-env contract), and align release automation checks.
- **Dependencies**:
  - Task 4.1
- **Complexity**: 7
- **Acceptance criteria**:
  - Version bump flow no longer regenerates zsh bundle.
  - `VERSIONS.env` removes `ZSH_KIT_REF` and keeps only active pin keys (`AGENT_KIT_REF`, and `AGENT_ENV_REF` if introduced).
  - `release_audit` verifies only active pin keys and fails on legacy pin keys.
- **Validation**:
  - `bash scripts/bump_versions.sh --help`
  - `bash scripts/release_prepare_changelog.sh --help`
  - `bash scripts/release_audit.sh --help`

### Task 4.4: Update CI and publish workflows
- **Location**:
  - `.github/workflows/ci.yml`
  - `.github/workflows/publish.yml`
- **Description**: Replace bundle/script-specific checks with Rust checks (`fmt`, `clippy`, `test`) plus updated smoke/e2e flows.
- **Dependencies**:
  - Task 4.2
  - Task 4.3
- **Complexity**: 6
- **Acceptance criteria**:
  - CI workflows pass with new command set.
  - Publish workflow passes required build args and metadata for Rust cutover.
- **Validation**:
  - `rg -n "cargo fmt|cargo clippy|cargo test|AWL_E2E" .github/workflows/ci.yml .github/workflows/publish.yml`
  - `! rg -n "generate_agent_workspace_bundle|agent-workspace.wrapper.zsh" .github/workflows/ci.yml .github/workflows/publish.yml`

## Sprint 5: Host `cws` -> `awl` Naming Migration (env + tests)
**Goal**: Complete naming migration on host wrappers, env vars, and automated tests.
**Demo/Validation**:
- Command(s): `rg -n "\bcws\b|\bCWS_" scripts tests DEVELOPMENT.md README.md docs/guides`
- Verify: Primary host/docs/tests paths use `awl`/`AWL_*` naming.

### Task 5.1: Rename host wrapper scripts and function names
- **Location**:
  - `scripts/cws.bash`
  - `scripts/cws.zsh`
  - `scripts/awl.bash`
  - `scripts/awl.zsh`
- **Description**: Migrate host wrappers to `awl` naming and `aw*` helper naming while preserving behavior.
- **Dependencies**:
  - Task 3.2
- **Complexity**: 6
- **Acceptance criteria**:
  - `awl` is the documented default command.
  - Legacy `cws` entrypoints are removed; no runtime shim is provided.
- **Validation**:
  - `bash -n scripts/awl.bash`
  - `zsh -n scripts/awl.zsh`
  - `test ! -e scripts/cws.bash && test ! -e scripts/cws.zsh`

### Task 5.2: Migrate env naming from `CWS_*` to `AWL_*`
- **Location**:
  - `scripts/awl.bash`
  - `scripts/awl.zsh`
  - `tests/conftest.py`
  - `tests/e2e/plan.py`
  - `DEVELOPMENT.md`
- **Description**: Rename environment variables used by host wrappers and tests (legacy `IMAGE`, `AUTH`, `DOCKER_ARGS`, `E2E*` prefixes) to `AWL_*` equivalents with hard cutover semantics (no legacy runtime fallback).
- **Dependencies**:
  - Task 5.1
- **Complexity**: 7
- **Acceptance criteria**:
  - Default docs/tests only mention `AWL_*`.
  - `CWS_*` runtime fallback is not implemented.
- **Validation**:
  - `rg -n "\bAWL_[A-Z0-9_]+\b" scripts tests DEVELOPMENT.md`
  - `! rg -n "\bCWS_[A-Z0-9_]+\b" scripts/awl.bash scripts/awl.zsh tests/conftest.py tests/e2e/plan.py README.md DEVELOPMENT.md docs/guides/awl`

### Task 5.3: Rename e2e/smoke test files and symbols to `awl`
- **Location**:
  - `tests/e2e/test_cws_cli_cases.py`
  - `tests/e2e/test_cws_cli_plan.py`
  - `tests/e2e/test_cws_bash_plan.py`
  - `tests/e2e/test_cws_zsh_plan.py`
  - `tests/e2e/test_awl_cli_cases.py`
  - `tests/e2e/test_awl_cli_plan.py`
  - `tests/e2e/test_awl_bash_plan.py`
  - `tests/e2e/test_awl_zsh_plan.py`
  - `tests/test_wrapper_equivalence.py`
  - `tests/script_specs/scripts/awl.bash.json`
  - `tests/script_specs/scripts/awl.zsh.json`
- **Description**: Rename test files/classes/case IDs and script specs so CI artifact names and selection gates are `awl`-native.
- **Dependencies**:
  - Task 5.2
- **Complexity**: 7
- **Acceptance criteria**:
  - Script smoke and e2e test modules no longer use `cws` identifiers as primary names.
  - Coverage reports and artifact paths use `awl` naming.
- **Validation**:
  - `.venv/bin/python -m pytest -m script_smoke`
  - `.venv/bin/python -m pytest --collect-only -m e2e`

## Sprint 6: Agent-env Integration and End-to-End Validation
**Goal**: Prove `agent-workspace` Rust CLI correctly drives `agent-env` in real execution paths.
**Demo/Validation**:
- Command(s): `AWL_E2E=1 AWL_E2E_CASE=help .venv/bin/python -m pytest -m e2e tests/e2e/test_awl_cli_cases.py`
- Verify: Real Docker e2e passes selected cases and validates contract integration.

### Task 6.1: Add agent-env contract integration tests
- **Location**:
  - `tests/e2e/test_agent_env_contract.py`
  - `tests/e2e/plan.py`
  - `docs/runbooks/INTEGRATION_TEST.md`
- **Description**: Add targeted checks that validate launcher-to-agent-env contract (workspace creation, repo path output, auth/tunnel behavior gates).
- **Dependencies**:
  - Task 3.1
  - Task 5.2
- **Complexity**: 8
- **Acceptance criteria**:
  - Tests assert expected behavior for create/exec/reset/rm flows against agent-env.
  - Contract drift produces actionable failure output.
- **Validation**:
  - `AWL_E2E=1 .venv/bin/python -m pytest -m e2e tests/e2e/test_agent_env_contract.py -q`

### Task 6.2: Add Rust integration tests for docker argv parity
- **Location**:
  - `crates/agent-workspace/tests/host_wrapper_parity.rs`
  - `crates/agent-workspace/tests/command_dispatch.rs`
- **Description**: Validate Rust argv generation and dispatch logic against parity matrix scenarios without real Docker dependency.
- **Dependencies**:
  - Task 3.2
  - Task 3.3
- **Complexity**: 6
- **Acceptance criteria**:
  - Rust tests cover token injection, env parsing, and shorthand mapping.
- **Validation**:
  - `cargo test -p agent-workspace --tests`

### Task 6.3: Run full project validation suite for cutover readiness
- **Location**:
  - `DEVELOPMENT.md`
  - `out/tests/e2e/summary.jsonl`
- **Description**: Execute required local checks and targeted e2e gates, capture artifacts, and record pass/fail status.
- **Dependencies**:
  - Task 6.1
  - Task 6.2
- **Complexity**: 5
- **Acceptance criteria**:
  - All required pre-submit checks pass.
  - At least one real Docker e2e path passes with `awl` naming.
- **Validation**:
  - `bash -n $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
  - `zsh -n $(git ls-files 'scripts/*.zsh')`
  - `shellcheck $(git ls-files 'scripts/*.sh' 'scripts/*.bash')`
  - `.venv/bin/python -m ruff format --check .`
  - `.venv/bin/python -m ruff check .`
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `.venv/bin/python -m pytest -m script_smoke`
  - `AWL_E2E=1 AWL_E2E_CASE=help .venv/bin/python -m pytest -m e2e tests/e2e/test_awl_cli_cases.py`

## Sprint 7: Documentation, Release Workflow, and Cutover
**Goal**: Publish a coherent user/developer workflow using Rust + awl naming.
**Demo/Validation**:
- Command(s): `rg -n "cws|CWS_|bundle-wrapper|generate_agent_workspace_bundle" README.md docs DEVELOPMENT.md scripts`
- Verify: Docs and runbooks align with cutover architecture and commands.

### Task 7.1: Migrate user docs and guides to `awl` naming
- **Location**:
  - `README.md`
  - `docs/BUILD.md`
  - `docs/DESIGN.md`
  - `docs/guides/README.md`
  - `docs/guides/cws/README.md`
  - `docs/guides/awl/README.md`
- **Description**: Rewrite installation/quickstart/reference docs to use `awl`/`aw*` commands and Rust architecture details.
- **Dependencies**:
  - Task 5.2
  - Task 4.2
- **Complexity**: 6
- **Acceptance criteria**:
  - Primary guide path is `docs/guides/awl/`.
  - README and BUILD examples use `AWL_*` env variables.
- **Validation**:
  - `rg -n "\bcws\b|\bCWS_" README.md docs/BUILD.md docs/guides docs/DESIGN.md`

### Task 7.2: Update developer runbooks and release docs
- **Location**:
  - `DEVELOPMENT.md`
  - `docs/runbooks/VERSION_BUMPS.md`
  - `docs/RELEASE_GUIDE.md`
  - `CHANGELOG.md`
- **Description**: Align required checks, e2e gate names, and version bump/release instructions with Rust cutover and awl naming.
- **Dependencies**:
  - Task 4.3
  - Task 6.3
- **Complexity**: 5
- **Acceptance criteria**:
  - Development guide commands are executable on the new toolchain.
  - Release docs no longer reference zsh bundle regeneration.
- **Validation**:
  - `rg -n "generate_agent_workspace_bundle|ZSH_KIT_REF|AWL_E2E|cargo" DEVELOPMENT.md docs/runbooks/VERSION_BUMPS.md docs/RELEASE_GUIDE.md`

### Task 7.3: Perform cutover dry-run and CI publish verification
- **Location**:
  - `.github/workflows/ci.yml`
  - `.github/workflows/publish.yml`
  - `out/tests/e2e/summary.jsonl`
- **Description**: Push migration branch and verify CI build/publish paths with new binary, then document cutover evidence.
- **Dependencies**:
  - Task 7.1
  - Task 7.2
- **Complexity**: 7
- **Acceptance criteria**:
  - CI passes on migration branch.
  - Publish workflow builds image with Rust binary entrypoint.
- **Validation**:
  - `git push origin HEAD`
  - `gh run watch "$(gh run list --workflow ci.yml --limit 1 --json databaseId -q '.[0].databaseId')" --exit-status`
  - `gh run watch "$(gh run list --workflow publish.yml --limit 1 --json databaseId -q '.[0].databaseId')" --exit-status`

## Testing Strategy
- Unit: Rust module tests for CLI parsing, env normalization, docker argv generation, shorthand mapping, and Codex env passthrough.
- Integration: Rust integration tests plus Python script-smoke tests for host wrapper invocation parity.
- E2E/manual: Docker-backed pytest e2e (`AWL_E2E=1`) covering CLI plan cases, plus targeted agent-env contract tests and local image smoke runs.

## Risks & gotchas
- Behavior drift risk: Replacing zsh bundle with Rust can subtly change edge-case semantics (`reset`, `auth`, `tunnel`).
- Contract skew risk: `agent-kit` low-level launcher still exposes `CODEX_*` internals; adapter layer must remain explicit.
- Migration blast radius: Renaming `cws`/`CWS_*` to `awl`/`AWL_*` touches scripts, tests, docs, and CI at once.
- Tooling split risk: CI now spans Rust + Python test stacks; failure reporting must remain clear.
- Hard-cutover risk: downstream automation still using `cws`/`CWS_*` will break immediately unless migrated before release.

## Rollback plan
1. Keep a rollback tag for the last zsh-bundle-based release and preserve publishable image tags before cutover.
2. If critical regressions appear, revert migration commits that remove bundle assets and restore prior Dockerfile entrypoint.
3. Re-enable legacy CI gates (`scripts/cws.*`, bundle smoke specs) on rollback branch to restore known-good checks.
4. Publish rollback image tags (`latest` and `sha-*`) from rollback branch and update README/runbook with temporary rollback notice.
5. Keep `awl` branch alive, fix forward under feature flags, then reattempt cutover with narrowed scope.
