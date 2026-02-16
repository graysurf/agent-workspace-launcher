# Plan: AWL Rust Native CLI Cutover to `agent-workspace-launcher`

## Overview
This plan migrates host-side `awl` behavior from shell-based `docker run` wrappers to a Rust-native executable path, with `agent-workspace-launcher` as the primary CLI name. `awl` remains supported as an alias only (argv0/symlink compatibility), not the primary implementation surface. The target outcome is that users can install and run the launcher binary directly without pulling `graysurf/agent-workspace-launcher` as a runtime dependency. Release and Homebrew packaging will be updated to ship the primary binary name and alias contract consistently.

## Scope
- In scope: Replace host wrapper runtime behavior currently implemented in `scripts/awl.bash` and `scripts/awl.zsh` with Rust implementation in `crates/agent-workspace`.
- In scope: Make `agent-workspace-launcher` the primary command identity in CLI help, release assets, and packaging.
- In scope: Keep `awl` as a compatibility alias (binary alias/symlink and docs).
- In scope: Update tests, CI, release workflows, and docs to validate the new command identity and runtime path.
- Out of scope: Rewriting archived progress docs under `docs/progress/archived/`.
- Out of scope: Renaming Codex compatibility environment variables (`CODEX_SECRET_DIR`, `CODEX_AUTH_FILE`).

## Assumptions (if any)
1. "No Docker dependency" in this cutover means no dependency on launching `docker run ... graysurf/agent-workspace-launcher:*` for the host entrypoint.
2. Workspace lifecycle operations may still use host container tooling in the first cutover milestone if required for parity; this is tracked as a risk and explicit follow-up decision.
3. Homebrew tap updates are delivered in the companion repo `~/Project/graysurf/homebrew-tap` as part of release rollout.

## Sprint 1: Contract Freeze and Migration Boundary
**Goal**: Lock the command identity and compatibility contract before implementation.
**Demo/Validation**:
- Command(s): `rg -n "agent-workspace-launcher|awl|AWL_IMAGE|AWL_DOCKER_ARGS" docs README.md DEVELOPMENT.md`
- Verify: The migration target and deprecation boundary are explicitly documented and reviewable.

### Task 1.1: Define CLI identity and alias contract
- **Location**:
  - `docs/specs/native-cli-contract.md`
  - `docs/specs/host-awl-alias-contract.md`
- **Description**: Specify that `agent-workspace-launcher` is the primary command, `awl` is alias-only, and command semantics (`auth/create/ls/rm/exec/reset/tunnel`) remain stable.
- **Dependencies**:
  - none
- **Complexity**: 3
- **Acceptance criteria**:
  - Contract docs explicitly define primary command name, alias behavior, and compatibility guarantees.
  - Contract docs include expected output/help naming conventions for both invocation names.
- **Validation**:
  - `rg -n "primary|alias|agent-workspace-launcher|awl" docs/specs/native-cli-contract.md docs/specs/host-awl-alias-contract.md`

### Task 1.2: Define runtime dependency boundary and deprecations
- **Location**:
  - `docs/specs/runtime-boundary.md`
  - `docs/DESIGN.md`
- **Description**: Document removal of host-side launcher-image dependency (`AWL_IMAGE`, `AWL_DOCKER_ARGS`, wrapper-driven `docker run`) and define any transitional compatibility policy.
- **Dependencies**:
  - Task 1.1
- **Complexity**: 5
- **Acceptance criteria**:
  - Runtime boundary doc states what is removed immediately vs temporarily supported with warnings.
  - `docs/DESIGN.md` no longer describes `awl` as a Docker-wrapper architecture for the primary path.
- **Validation**:
  - `rg -n "AWL_IMAGE|AWL_DOCKER_ARGS|docker run|primary path" docs/specs/runtime-boundary.md docs/DESIGN.md`

### Task 1.3: Create file-touch inventory and sequencing map
- **Location**:
  - `docs/plans/aw-awl-migration-inventory.md`
- **Description**: Build a complete inventory of affected runtime, test, release, and docs files; include minimal safe sequencing and high-risk edges.
- **Dependencies**:
  - Task 1.1
  - Task 1.2
- **Complexity**: 4
- **Acceptance criteria**:
  - Inventory covers at least: `scripts/awl.*`, `crates/agent-workspace/*`, e2e/script-smoke tests, release workflows, and release docs.
  - Inventory identifies order constraints and candidate parallel edits.
- **Validation**:
  - `rg -n "scripts/awl|crates/agent-workspace|release-brew|release-docker|test_awl|script_specs" docs/plans/aw-awl-migration-inventory.md`

## Sprint 2: Binary Identity and Alias Mechanics
**Goal**: Make `agent-workspace-launcher` the canonical executable and keep `awl` alias semantics.
**Demo/Validation**:
- Command(s): `cargo fmt --all -- --check && cargo check --workspace`
- Verify: The build outputs and CLI metadata reflect the new primary command identity.

### Task 2.1: Rename binary target and CLI metadata
- **Location**:
  - `crates/agent-workspace/Cargo.toml`
  - `crates/agent-workspace/src/main.rs`
  - `crates/agent-workspace/src/cli.rs`
  - `README.md`
- **Description**: Rename the exported binary target to `agent-workspace-launcher` and update clap metadata/help strings accordingly.
- **Dependencies**:
  - Task 1.1
- **Complexity**: 4
- **Acceptance criteria**:
  - `cargo build --release -p agent-workspace` produces `agent-workspace-launcher` binary artifact.
  - `--help` output and docs show `agent-workspace-launcher` as the primary invocation name.
- **Validation**:
  - `cargo build --release -p agent-workspace`
  - `target/release/agent-workspace-launcher --help`

### Task 2.2: Add argv0 alias behavior for `awl`
- **Location**:
  - `crates/agent-workspace/src/lib.rs`
  - `crates/agent-workspace/src/cli.rs`
  - `crates/agent-workspace/src/alias.rs`
- **Description**: Detect invocation name (`argv[0]`) and preserve `awl` compatibility semantics without duplicating command implementations.
- **Dependencies**:
  - Task 2.1
- **Complexity**: 6
- **Acceptance criteria**:
  - Invoking via symlink/executable name `awl` routes to the same command tree and runtime behavior.
  - Command identity in logs/help remains deterministic and documented.
- **Validation**:
  - `tmp="$(mktemp -d)"; ln -s "$(pwd)/target/release/agent-workspace-launcher" "$tmp/awl"; "$tmp/awl" --help`
  - `cargo test -p agent-workspace alias::tests`

### Task 2.3: Replace shell wrappers with thin compatibility shims
- **Location**:
  - `scripts/awl.bash`
  - `scripts/awl.zsh`
  - `tests/script_specs/scripts/awl.bash.json`
  - `tests/script_specs/scripts/awl.zsh.json`
- **Description**: Reduce wrapper scripts to alias/completion shims that exec the Rust binary and remove embedded runtime logic.
- **Dependencies**:
  - Task 2.2
- **Complexity**: 5
- **Acceptance criteria**:
  - Wrapper scripts do not contain `docker run` execution logic.
  - Existing sourcing/alias ergonomics remain functional for bash and zsh users.
- **Validation**:
  - `! rg -n "docker[[:space:]]+run|AWL_IMAGE|AWL_DOCKER_ARGS" scripts/awl.bash scripts/awl.zsh`
  - `.venv/bin/python -m pytest -m script_smoke tests/test_script_smoke.py -k awl`

## Sprint 3: Rust Runtime Cutover (Host Entry)
**Goal**: Move host-side runtime behavior into Rust and remove launcher-image runtime dependency.
**Demo/Validation**:
- Command(s): `cargo test -p agent-workspace`
- Verify: Host invocation flow works through Rust implementation without `docker run` wrapper path.

### Task 3.1: Eliminate hard dependency on external low-level launcher path
- **Location**:
  - `crates/agent-workspace/src/launcher.rs`
  - `crates/agent-workspace/src/runtime.rs`
  - `crates/agent-workspace/src/commands/mod.rs`
  - `crates/agent-workspace/src/commands/create.rs`
  - `crates/agent-workspace/src/commands/ls.rs`
  - `crates/agent-workspace/src/commands/rm.rs`
  - `crates/agent-workspace/src/commands/exec.rs`
  - `crates/agent-workspace/src/commands/reset.rs`
  - `crates/agent-workspace/src/commands/tunnel.rs`
- **Description**: Replace or gate `resolve_launcher_path()` forwarding so primary flows no longer fail when `/opt/agent-kit/docker/agent-env/bin/agent-workspace` is absent.
- **Dependencies**:
  - Task 1.2
  - Task 2.1
- **Complexity**: 9
- **Acceptance criteria**:
  - Running `agent-workspace-launcher ls` on a host install no longer errors with "launcher not found".
  - Core command paths execute through Rust-owned runtime handlers.
- **Validation**:
  - `unset AGENT_WORKSPACE_LAUNCHER; target/release/agent-workspace-launcher ls || true`
  - `cargo test -p agent-workspace launcher::tests`

### Task 3.2: Port host auth/token injection policy into Rust
- **Location**:
  - `crates/agent-workspace/src/github_auth.rs`
  - `crates/agent-workspace/src/runtime.rs`
  - `crates/agent-workspace/src/launcher.rs`
- **Description**: Port wrapper-only auth logic (`AWL_AUTH=auto|env|none`, host `gh` keyring probing, token pass-through rules) into Rust runtime.
- **Dependencies**:
  - Task 3.1
- **Complexity**: 7
- **Acceptance criteria**:
  - `create/reset/auth github` follow identical auth precedence and fallback behavior as documented.
  - Behavior is unit-testable without requiring online GitHub access in every test.
- **Validation**:
  - `cargo test -p agent-workspace github_auth::tests`
  - `cargo test -p agent-workspace runtime::tests`

### Task 3.3: Remove wrapper-image env contract from runtime entry
- **Location**:
  - `crates/agent-workspace/src/cli.rs`
  - `crates/agent-workspace/src/env.rs`
  - `README.md`
  - `docs/guides/awl/11-reference.md`
- **Description**: Remove or deprecate `AWL_IMAGE` and `AWL_DOCKER_ARGS` from active runtime path and replace with native CLI config contract.
- **Dependencies**:
  - Task 3.1
  - Task 3.2
- **Complexity**: 6
- **Acceptance criteria**:
  - Runtime behavior does not depend on `AWL_IMAGE` to execute commands.
  - User docs clearly explain native invocation and any deprecation warnings.
- **Validation**:
  - `! rg -n "AWL_IMAGE|AWL_DOCKER_ARGS" crates/agent-workspace/src`
  - `rg -n "AWL_IMAGE|AWL_DOCKER_ARGS|deprecated" README.md docs/guides/awl/11-reference.md`

### Task 3.4: Rebase e2e/smoke harness on native binary execution
- **Location**:
  - `tests/e2e/plan.py`
  - `tests/e2e/test_awl_cli_cases.py`
  - `tests/conftest.py`
  - `tests/test_wrapper_equivalence.py`
- **Description**: Switch test harness defaults from launcher-image invocation to native binary invocation while preserving deterministic cleanup and opt-in destructive gates.
- **Dependencies**:
  - Task 2.3
  - Task 3.1
- **Complexity**: 8
- **Acceptance criteria**:
  - Script-smoke and e2e harness no longer assume `graysurf/agent-workspace-launcher:latest` pull path.
  - Tests include coverage for both `agent-workspace-launcher` and `awl` alias invocation.
- **Validation**:
  - `.venv/bin/python -m pytest -m "not e2e" -k "awl or launcher"`
  - `AWL_E2E=1 AWL_E2E_CASE=help .venv/bin/python -m pytest -m e2e tests/e2e/test_awl_cli_cases.py`

## Sprint 4: Release and Packaging Realignment
**Goal**: Ship CLI-first release artifacts where `agent-workspace-launcher` is primary and `awl` is alias.
**Demo/Validation**:
- Command(s): `rg -n "agent-workspace-launcher|awl" .github/workflows docs/RELEASE_GUIDE.md docs/runbooks/VERSION_BUMPS.md`
- Verify: Release automation and docs are aligned with binary-first distribution.

### Task 4.1: Update release workflow to package native binaries + alias
- **Location**:
  - `.github/workflows/release-brew.yml`
  - `.github/workflows/release-docker.yml`
  - `scripts/release_audit.sh`
- **Description**: Build target binaries, package `bin/agent-workspace-launcher` and `bin/awl` alias in tarballs, and keep checksum publication for each target. If a dedicated CLI workflow is added, create it explicitly and make it the source of truth in release docs.
- **Dependencies**:
  - Task 2.1
  - Task 3.1
- **Complexity**: 8
- **Acceptance criteria**:
  - Per-target archive contains executable primary binary plus alias entry.
  - Release audit enforces the new payload contract.
- **Validation**:
  - `release_tag="${RELEASE_TAG:?set RELEASE_TAG (for example v1.2.3)}"; tar -tzf "dist/agent-workspace-launcher-${release_tag}-aarch64-apple-darwin.tar.gz" | rg -n "bin/agent-workspace-launcher|bin/awl"`
  - `release_tag="${RELEASE_TAG:?set RELEASE_TAG (for example v1.2.3)}"; bash scripts/release_audit.sh --version "${release_tag}" --strict`

### Task 4.2: Update Homebrew formula install contract (tap repo)
- **Location**:
  - `~/Project/graysurf/homebrew-tap/Formula/agent-workspace-launcher.rb`
  - `docs/RELEASE_GUIDE.md`
- **Description**: Change formula install logic to install `agent-workspace-launcher` as primary executable and create `awl` alias symlink/wrapper.
- **Dependencies**:
  - Task 4.1
- **Complexity**: 6
- **Acceptance criteria**:
  - `brew install agent-workspace-launcher` provides both commands, with `agent-workspace-launcher` as canonical.
  - Formula test validates at least `agent-workspace-launcher --help` and `awl --help`.
- **Validation**:
  - `ruby -c ~/Project/graysurf/homebrew-tap/Formula/agent-workspace-launcher.rb`
  - `HOMEBREW_NO_AUTO_UPDATE=1 brew test agent-workspace-launcher`

### Task 4.3: Align release/runbook docs with CLI-first channel
- **Location**:
  - `docs/RELEASE_GUIDE.md`
  - `docs/runbooks/VERSION_BUMPS.md`
  - `README.md`
- **Description**: Update release steps and verification commands to treat binary artifacts as the primary end-user path; keep Docker channel policy explicit and non-primary.
- **Dependencies**:
  - Task 4.1
- **Complexity**: 5
- **Acceptance criteria**:
  - Release guide validates binary assets and alias behavior explicitly.
  - Documentation no longer presents Docker wrapper runtime as the main user entrypoint.
- **Validation**:
  - `rg -n "docker run --rm -it graysurf/agent-workspace-launcher|AWL_IMAGE|primary" docs/RELEASE_GUIDE.md docs/runbooks/VERSION_BUMPS.md README.md`

## Sprint 5: Docs, CI, and Rollout Hardening
**Goal**: Complete migration quality gates and operator guidance for safe rollout.
**Demo/Validation**:
- Command(s): `bash -n $(git ls-files 'scripts/*.sh' 'scripts/*.bash') && zsh -n $(git ls-files 'scripts/*.zsh')`
- Verify: CI/test/docs baselines are updated and green for the new runtime path.

### Task 5.1: Rewrite user guides to binary-first usage
- **Location**:
  - `docs/guides/awl/README.md`
  - `docs/guides/awl/01-install.md`
  - `docs/guides/awl/12-agent-workspace.md`
  - `docs/BUILD.md`
- **Description**: Replace wrapper-first and direct `docker run` guidance with `agent-workspace-launcher` primary usage and `awl` alias examples.
- **Dependencies**:
  - Task 3.3
  - Task 4.3
- **Complexity**: 6
- **Acceptance criteria**:
  - Quickstart flow succeeds with direct binary invocation.
  - `awl` examples are clearly marked as alias ergonomics, not core runtime dependency.
- **Validation**:
  - `rg -n "docker run --rm -it graysurf/agent-workspace-launcher:latest|AWL_IMAGE" docs/guides/awl docs/BUILD.md`
  - `rg -n "agent-workspace-launcher|alias" docs/guides/awl/README.md docs/guides/awl/01-install.md`
  - `agent-workspace-launcher --help && awl --help`

### Task 5.2: Update CI/pre-submit checks for native launcher path
- **Location**:
  - `DEVELOPMENT.md`
  - `.github/workflows/ci.yml`
  - `tests/test_script_smoke.py`
- **Description**: Ensure mandatory checks and CI jobs verify the native binary path and alias behavior, without requiring wrapper-image pull for smoke.
- **Dependencies**:
  - Task 3.4
  - Task 4.1
- **Complexity**: 6
- **Acceptance criteria**:
  - CI includes native launcher smoke checks.
  - Pre-submit checklist reflects new runtime assumptions and commands.
- **Validation**:
  - `rg -n "agent-workspace-launcher|awl --help|script_smoke" DEVELOPMENT.md .github/workflows/ci.yml`
  - `.venv/bin/python -m pytest -m script_smoke`

### Task 5.3: Publish migration notes and compatibility policy
- **Location**:
  - `CHANGELOG.md`
  - `README.md`
  - `docs/guides/awl/10-troubleshooting.md`
- **Description**: Add migration notes for removed wrapper-image behavior, alias expectations, and fallback/compat windows.
- **Dependencies**:
  - Task 5.1
  - Task 5.2
- **Complexity**: 4
- **Acceptance criteria**:
  - Users can map old env/config usage to new behavior with explicit examples.
  - Troubleshooting covers missing binary, alias setup, and runtime backend expectations.
- **Validation**:
  - `rg -n "migration|deprecated|alias|agent-workspace-launcher" CHANGELOG.md README.md docs/guides/awl/10-troubleshooting.md`

## Sprint 6: Release Readiness and Cutover Verification
**Goal**: Execute final validation and publish with reversible rollout safeguards.
**Demo/Validation**:
- Command(s): `.venv/bin/python -m pytest -m script_smoke && cargo test -p agent-workspace`
- Verify: All mandatory checks pass and release rehearsal confirms binary-first contract.

### Task 6.1: Run full mandatory check suite and minimal e2e
- **Location**:
  - `DEVELOPMENT.md`
  - `tests/e2e/test_awl_cli_cases.py`
- **Description**: Execute required lint/format/test gates plus minimal e2e coverage for primary/alias invocation before tagging.
- **Dependencies**:
  - Task 5.2
- **Complexity**: 7
- **Acceptance criteria**:
  - `DEVELOPMENT.md` required checks pass in CI-equivalent environment.
  - Minimal e2e case validates direct binary invocation path.
- **Validation**:
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

### Task 6.2: Release rehearsal and rollback drill
- **Location**:
  - `docs/RELEASE_GUIDE.md`
  - `docs/runbooks/VERSION_BUMPS.md`
  - `~/Project/graysurf/homebrew-tap/Formula/agent-workspace-launcher.rb`
- **Description**: Rehearse release + install verification using pre-release artifacts and execute rollback drill procedure to verify operator readiness.
- **Dependencies**:
  - Task 6.1
- **Complexity**: 7
- **Acceptance criteria**:
  - Rehearsal confirms installable binary asset with `agent-workspace-launcher` and `awl`.
  - Rollback drill can restore previous working formula/version within one operator session.
- **Validation**:
  - `release_tag="${RELEASE_TAG:?set RELEASE_TAG (for example v1.2.3)}"; gh release view "${release_tag}" --json assets --jq '.assets[].name'`
  - `brew upgrade agent-workspace-launcher || brew install agent-workspace-launcher`
  - `agent-workspace-launcher --help && awl --help`

## Dependency & Parallelism Map
- Sprint 1 parallel batch: Task 1.1 can run first, then Tasks 1.2 and 1.3 can run in parallel.
- Sprint 2 parallel batch: Task 2.1 first; Tasks 2.2 and completion-related prep can run in parallel once binary identity is stable.
- Sprint 3 parallel batch: Task 3.1 is the critical path; Tasks 3.2 and 3.4 can overlap after core runtime seams exist.
- Sprint 4 parallel batch: Task 4.1 first; Tasks 4.2 and 4.3 can run in parallel after artifact contract lands.
- Sprint 5 parallel batch: Tasks 5.1 and 5.2 can run in parallel with coordination on shared docs snippets; Task 5.3 follows.
- Sprint 6 sequence: Task 6.1 then Task 6.2 (no parallelization).

## Testing Strategy
- Unit: Expand Rust module-level tests for alias routing, auth policy, env parsing, and command dispatch behavior.
- Integration: Add CLI integration tests that invoke compiled binaries by both names (`agent-workspace-launcher`, `awl`) and assert equivalent behavior.
- E2E/manual: Keep minimal opt-in e2e coverage for create/ls/exec/rm lifecycle with explicit cleanup gates.

## Risks & gotchas
- Ambiguity risk: "No Docker dependency" can mean "no launcher image dependency" or "no container backend dependency". This plan removes launcher-image dependency first and documents backend policy explicitly.
- Runtime parity risk: Current flows rely on low-level launcher behavior in `agent-kit`; replacing forwarding with Rust logic can regress edge cases in auth/reset/tunnel.
- Packaging risk: Brew formula and release asset contracts currently ship scripts only; introducing binary + alias requires coordinated repo and tap updates.
- Backward-compat risk: Existing users may rely on `AWL_IMAGE` and `AWL_DOCKER_ARGS`; migration needs warnings and deterministic fallback behavior.
- CI risk: Test fixtures currently assert `docker run` strings; harness changes must avoid false positives and preserve coverage signal.

## Rollback plan
- Keep previous stable release artifacts and formula commit SHA available; if rollout fails, revert tap formula to prior version and republish checksums from prior tag.
- Preserve a short compatibility window where thin shell wrappers can still exec the prior path if the native binary is missing, then remove after confidence criteria are met.
- If Rust runtime parity breaks critical commands (`create`, `exec`, `rm`, `reset`), revert the cutover PR and re-enable old wrapper execution path via a guarded feature flag.
- Run post-rollback verification: `brew reinstall agent-workspace-launcher`, `awl --help`, and one smoke workspace lifecycle command to confirm recovery.
