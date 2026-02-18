# Plan: Enable AWL dual runtime with container default

## Overview
This plan introduces a dual-runtime architecture for `agent-workspace-launcher`/`awl`: `container` and `host`, with `container` as the default runtime. The CLI surface remains stable (`create`, `ls`, `exec`, `rm`, `reset`, `auth`, `tunnel`) while dispatch behavior becomes runtime-aware. Existing host-native behavior remains available via explicit runtime selection for local fallback and gradual migration. Documentation will be updated end-to-end so README and guides describe the new default and runtime selection model consistently.

## Scope
- In scope: Add runtime-selection contract (`--runtime`, env default, deterministic resolution) with `container` default.
- In scope: Implement a Rust `container` backend for lifecycle commands using host Docker daemon.
- In scope: Keep `host` backend functional and explicitly selectable.
- In scope: Update CLI docs/specs/guides/README and troubleshooting for dual runtime.
- In scope: Extend tests for runtime routing, parsing, and smoke coverage for both runtimes.
- Out of scope: Kubernetes/Podman backend support.
- Out of scope: Full removal of `awl_docker` wrapper in this phase.
- Out of scope: Remote multi-host orchestration beyond local Docker daemon semantics.

## Assumptions (if any)
1. Docker is available on hosts that use the default `container` runtime.
2. Existing `agent-env` image contract remains valid (`graysurf/agent-env:*`) and can be reused by Rust container backend.
3. Runtime selection precedence is deterministic and documented (flag > env > default).
4. Backward compatibility keeps host-native workflows viable via explicit `--runtime host` (or env override).

## Sprint 1: Runtime Contract and Dispatch Foundation
**Goal**: Establish dual-runtime contract, runtime resolver, and dispatch scaffolding before command rewrites.
**Demo/Validation**:
- Command(s): `cargo test -p agent-workspace runtime::tests`
- Verify: Runtime resolution is deterministic and defaults to `container` when no override is provided.

### Task 1.1: Define runtime contract and compatibility policy
- **Location**:
  - `docs/specs/host-awl-contract.md`
  - `docs/specs/agent-workspace-rust-contract.md`
  - `docs/DESIGN.md`
- **Description**: Replace host-only runtime wording with dual-runtime contract. Specify default runtime (`container`), selection precedence, environment variables, and compatibility expectations for `awl` alias behavior.
- **Dependencies**:
  - none
- **Complexity**: 4
- **Acceptance criteria**:
  - Specs define `host` and `container` runtime semantics with explicit default and override precedence.
  - Design doc includes backend boundary and error behavior when Docker is unavailable.
- **Validation**:
  - `rg -n "runtime|container default|--runtime|AGENT_WORKSPACE_RUNTIME" docs/specs/host-awl-contract.md docs/specs/agent-workspace-rust-contract.md docs/DESIGN.md`

### Task 1.2: Add runtime resolver and dispatch abstraction in Rust
- **Location**:
  - `crates/agent-workspace/src/lib.rs`
  - `crates/agent-workspace/src/launcher.rs`
  - `crates/agent-workspace/src/cli.rs`
  - `crates/agent-workspace/src/runtime.rs` (new)
- **Description**: Introduce runtime enum/resolver (`flag`, env, default), runtime-aware dispatch entrypoint, and shared command execution contract used by both backends.
- **Dependencies**:
  - Task 1.1
- **Complexity**: 7
- **Acceptance criteria**:
  - All subcommands route through one runtime resolver.
  - Default runtime is `container`; explicit `--runtime host` and env override are honored.
  - Unknown runtime values fail with clear error and hint text.
- **Validation**:
  - `cargo test -p agent-workspace runtime::tests`
  - `cargo test -p agent-workspace cli::tests`

### Task 1.3: Extend CLI parsing/completion for runtime controls
- **Location**:
  - `crates/agent-workspace/src/cli.rs`
  - `scripts/awl.bash`
  - `scripts/awl.zsh`
  - `completions/` (if generated assets are tracked)
- **Description**: Add runtime options to top-level parsing and completion hints, while keeping backward-compatible invocation forms.
- **Dependencies**:
  - Task 1.2
- **Complexity**: 5
- **Acceptance criteria**:
  - `--runtime {container,host}` is recognized where documented.
  - Completion scripts suggest runtime options without regressing existing subcommand completion.
- **Validation**:
  - `.venv/bin/python -m pytest tests/test_wrapper_equivalence.py`
  - `bash -n $(git ls-files 'scripts/*.bash') && zsh -n $(git ls-files 'scripts/*.zsh')`

### Task 1.4: Add migration warnings and explicit fallback messaging
- **Location**:
  - `crates/agent-workspace/src/runtime.rs` (new)
  - `crates/agent-workspace/src/launcher.rs`
- **Description**: Add user-facing warnings for Docker-unavailable default path and provide one-step fallback guidance (`--runtime host` or env override) to reduce breakage during cutover.
- **Dependencies**:
  - Task 1.2
- **Complexity**: 3
- **Acceptance criteria**:
  - Docker unavailability errors include actionable fallback command.
  - Non-interactive flows return stable non-zero exit codes.
- **Validation**:
  - `cargo test -p agent-workspace runtime::tests -- --nocapture`

## Sprint 2: Container Backend Lifecycle (create/ls/exec/rm)
**Goal**: Deliver container runtime MVP that supports fast workspace provisioning via `agent-env` across hosts.
**Demo/Validation**:
- Command(s): run `create -> ls -> exec -> rm` using `--runtime container` and default runtime.
- Verify: Workspace lifecycle operations target Docker containers/volumes, not host directory skeletons.

### Task 2.1: Implement container backend primitives
- **Location**:
  - `crates/agent-workspace/src/runtime/container/mod.rs` (new)
  - `crates/agent-workspace/src/runtime/container/docker.rs` (new)
  - `crates/agent-workspace/src/runtime/container/naming.rs` (new)
- **Description**: Add Docker command helpers (`inspect`, `run`, `exec`, `rm`, volume operations), workspace name normalization, label conventions, and status probes.
- **Dependencies**:
  - Task 1.2
- **Complexity**: 8
- **Acceptance criteria**:
  - Container backend can create/find/remove runtime entities using deterministic naming and labels.
  - Helpers encapsulate shelling to Docker with robust error propagation.
- **Validation**:
  - `cargo test -p agent-workspace container::tests`

### Task 2.2: Implement `create` for container runtime with `agent-env` image contract
- **Location**:
  - `crates/agent-workspace/src/runtime/container/create.rs` (new)
  - `crates/agent-workspace/src/launcher/create.rs` (runtime split)
- **Description**: Port the existing `agent-env` create/up behavior into Rust container backend (container + volumes + optional clone/setup flags), keeping compatibility with documented Docker image contracts in this repository.
- **Dependencies**:
  - Task 2.1
- **Complexity**: 9
- **Acceptance criteria**:
  - `create` in default mode creates runnable workspace container and expected volumes.
  - Supports image override/no-pull semantics and no-clone mode with clear validation errors.
- **Validation**:
  - `cargo test -p agent-workspace container_create::tests`
  - `AWL_E2E=1 .venv/bin/python -m pytest tests/e2e/test_awl_cli_cases.py -k "create"`

### Task 2.3: Implement `ls`, `exec`, and `rm` for container runtime
- **Location**:
  - `crates/agent-workspace/src/runtime/container/ls.rs` (new)
  - `crates/agent-workspace/src/runtime/container/exec.rs` (new)
  - `crates/agent-workspace/src/runtime/container/rm.rs` (new)
  - `crates/agent-workspace/src/launcher/ls.rs`
  - `crates/agent-workspace/src/launcher/exec.rs`
  - `crates/agent-workspace/src/launcher/rm.rs`
- **Description**: Implement runtime-specific behavior for list/exec/remove, including remove-all semantics, volume retention flag policy, and structured output parity.
- **Dependencies**:
  - Task 2.1
  - Task 2.2
- **Complexity**: 8
- **Acceptance criteria**:
  - `ls` distinguishes runtime and reports container workspace entries.
  - `exec` opens shell/command inside workspace container.
  - `rm` supports targeted and bulk cleanup with confirmation controls.
- **Validation**:
  - `cargo test -p agent-workspace container_lifecycle::tests`
  - `AWL_E2E=1 .venv/bin/python -m pytest tests/e2e/test_awl_cli_cases.py -k "ls or exec or rm"`

### Task 2.4: Introduce mixed-runtime workspace resolution safeguards
- **Location**:
  - `crates/agent-workspace/src/launcher.rs`
  - `crates/agent-workspace/src/runtime.rs`
- **Description**: Prevent ambiguous targeting when a host workspace and container workspace share the same logical name; enforce runtime-scoped resolution and precise error hints.
- **Dependencies**:
  - Task 2.3
- **Complexity**: 6
- **Acceptance criteria**:
  - Ambiguous names do not silently target wrong backend.
  - Error output recommends explicit `--runtime` or normalized name.
- **Validation**:
  - `cargo test -p agent-workspace resolve_workspace::tests`

## Sprint 3: Container Parity for auth/reset/tunnel + Test Harness Upgrade
**Goal**: Complete runtime parity for remaining commands and modernize test coverage for dual backend behavior.
**Demo/Validation**:
- Command(s): run command matrix for `auth`, `reset`, `tunnel` in both runtimes.
- Verify: Dual runtime behavior is explicit, validated, and non-regressing for host mode.

### Task 3.1: Implement container backend for `auth` providers
- **Location**:
  - `crates/agent-workspace/src/runtime/container/auth.rs` (new)
  - `crates/agent-workspace/src/launcher/auth.rs`
- **Description**: Add container-scoped auth operations (GitHub, Codex, GPG) with runtime-appropriate storage and token/key handling, while preserving provider options.
- **Dependencies**:
  - Task 2.1
- **Complexity**: 8
- **Acceptance criteria**:
  - `auth github/codex/gpg` works in container runtime and keeps host runtime behavior intact.
  - Provider errors include runtime context.
- **Validation**:
  - `cargo test -p agent-workspace auth::tests`

### Task 3.2: Implement container backend for `reset` repo flows
- **Location**:
  - `crates/agent-workspace/src/runtime/container/reset.rs` (new)
  - `crates/agent-workspace/src/launcher/reset.rs`
- **Description**: Port reset subcommands to execute within workspace containers for container runtime, including depth/ref handling and confirmation semantics.
- **Dependencies**:
  - Task 2.3
- **Complexity**: 7
- **Acceptance criteria**:
  - `reset repo/work-repos/opt-repos/private-repo` succeeds in container runtime with expected path semantics.
  - Host-native reset behavior remains unchanged when runtime is `host`.
- **Validation**:
  - `cargo test -p agent-workspace reset::tests`
  - `AWL_E2E=1 .venv/bin/python -m pytest tests/e2e/test_awl_cli_cases.py -k "reset"`

### Task 3.3: Implement container backend for `tunnel`
- **Location**:
  - `crates/agent-workspace/src/runtime/container/tunnel.rs` (new)
  - `crates/agent-workspace/src/launcher/tunnel.rs`
- **Description**: Implement tunnel behavior as `docker exec ... code tunnel ...` with detach/json/name support and clear logging hints for first-time auth/device flow.
- **Dependencies**:
  - Task 2.3
- **Complexity**: 6
- **Acceptance criteria**:
  - `tunnel` supports foreground, detach, name override, and JSON mode in container runtime.
  - Host-native tunnel continues to run local `code tunnel`.
- **Validation**:
  - `cargo test -p agent-workspace tunnel::tests`
  - `AWL_E2E=1 CWS_E2E_ENABLE_TUNNEL=1 .venv/bin/python -m pytest tests/e2e/test_awl_cli_cases.py -k "tunnel"`

### Task 3.4: Rework e2e plan generator for runtime-aware cases
- **Location**:
  - `tests/e2e/plan.py`
  - `tests/e2e/test_awl_cli_plan.py`
  - `tests/e2e/test_awl_cli_cases.py`
  - `tests/stubs/bin/docker`
- **Description**: Update the e2e matrix to cover both runtime modes, include default-container expectations, and add deterministic stubs for CI-safe container routing verification.
- **Dependencies**:
  - Task 2.4
  - Task 3.1
  - Task 3.2
  - Task 3.3
- **Complexity**: 7
- **Acceptance criteria**:
  - CI matrix validates routing logic and command contract for both runtimes.
  - Runtime defaults are asserted by tests, not only docs.
- **Validation**:
  - `.venv/bin/python -m pytest tests/e2e/test_awl_cli_plan.py tests/e2e/test_awl_cli_cases.py`

## Sprint 4: Docs, Migration UX, and Release Readiness
**Goal**: Make all user-facing docs and references accurately reflect dual runtime with container default, plus safe migration guidance.
**Demo/Validation**:
- Command(s): `rg -n "host-native|container|runtime|AWL_DOCKER|awl_docker|default" README.md docs/guides docs/specs`
- Verify: No contradictory runtime messaging remains; default behavior and fallback are clear everywhere.

### Task 4.1: Rewrite top-level README and install docs for new default
- **Location**:
  - `README.md`
  - `docs/guides/01-install.md`
  - `docs/guides/02-quickstart.md`
- **Description**: Update command examples and installation guidance so primary path reflects default container runtime, with explicit host-runtime opt-out examples.
- **Dependencies**:
  - Task 1.1
  - Task 2.2
- **Complexity**: 5
- **Acceptance criteria**:
  - README quickstart works with default runtime assumptions.
  - Install guide clearly distinguishes default container path and host fallback.
- **Validation**:
  - `rg -n "default runtime|--runtime host|--runtime container|create|exec|rm" README.md docs/guides/01-install.md docs/guides/02-quickstart.md`

### Task 4.2: Update all command guides/reference/troubleshooting for dual runtime
- **Location**:
  - `docs/guides/03-create.md`
  - `docs/guides/04-exec.md`
  - `docs/guides/05-rm.md`
  - `docs/guides/06-reset.md`
  - `docs/guides/07-tunnel.md`
  - `docs/guides/08-auth.md`
  - `docs/guides/09-dood-rules.md`
  - `docs/guides/10-troubleshooting.md`
  - `docs/guides/11-reference.md`
  - `docs/guides/12-agent-workspace.md`
  - `docs/guides/README.md`
- **Description**: Normalize terminology and examples across guides: default container workflow, host override, runtime-specific caveats, and migration from prior host-default behavior.
- **Dependencies**:
  - Task 4.1
- **Complexity**: 6
- **Acceptance criteria**:
  - Every command guide documents runtime-specific differences where behavior diverges.
  - Troubleshooting covers Docker-unavailable default path and host fallback.
- **Validation**:
  - `rg -n "host-native only|container default|--runtime|AGENT_WORKSPACE_RUNTIME|awl_docker" docs/guides`

### Task 4.3: Update release/runbook docs and compatibility notes
- **Location**:
  - `docs/DOCKERHUB_REPOSITORY_OVERVIEW.md`
  - `docs/runbooks/INTEGRATION_TEST.md`
  - `docs/specs/agent-workspace-parity-matrix.md`
  - `CHANGELOG.md`
- **Description**: Align release verification and parity scope with dual runtime support; ensure release docs validate default container behavior and host fallback explicitly.
- **Dependencies**:
  - Task 3.4
  - Task 4.2
- **Complexity**: 4
- **Acceptance criteria**:
  - Integration runbook includes dual-runtime smoke steps.
  - Parity matrix includes container backend in-scope checks.
- **Validation**:
  - `rg -n "container backend|runtime parity|host fallback|integration smoke" docs/runbooks/INTEGRATION_TEST.md docs/specs/agent-workspace-parity-matrix.md docs/DOCKERHUB_REPOSITORY_OVERVIEW.md CHANGELOG.md`

### Task 4.4: Execute full pre-submit gates and runtime smoke matrix
- **Location**:
  - `DEVELOPMENT.md`
  - `tests/e2e/test_awl_cli_cases.py`
  - `tests/e2e/test_awl_cli_plan.py`
  - `tests/test_script_smoke.py`
- **Description**: Run all required repo checks plus a runtime smoke matrix (`default/container/host`) and record pass/fail outcomes for release confidence.
- **Dependencies**:
  - Task 3.4
  - Task 4.3
- **Complexity**: 5
- **Acceptance criteria**:
  - Required checks from `DEVELOPMENT.md` pass.
  - Manual/runtime smoke outputs are captured in a reproducible command list.
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
  - `tmp_home="$(mktemp -d)"; AGENT_WORKSPACE_HOME="${tmp_home}/workspaces" ./target/release/agent-workspace-launcher create --no-work-repos --name ws-default-smoke`
  - `tmp_home="$(mktemp -d)"; AGENT_WORKSPACE_HOME="${tmp_home}/workspaces" ./target/release/agent-workspace-launcher --runtime container create --no-work-repos --name ws-container-smoke`
  - `tmp_home="$(mktemp -d)"; AGENT_WORKSPACE_HOME="${tmp_home}/workspaces" ./target/release/agent-workspace-launcher --runtime host create --no-work-repos --name ws-host-smoke`

## Parallelization notes
- After Task 1.2, Task 1.3 and Task 1.4 can run in parallel.
- In Sprint 2, Task 2.2 and Task 2.3 can split across sub-teams after Task 2.1.
- In Sprint 3, Task 3.1/3.2/3.3 are parallelizable once Sprint 2 is stable; Task 3.4 should start after interfaces settle.
- In Sprint 4, docs updates (Task 4.1/4.2) can proceed in parallel with release/runbook alignment prep, but Task 4.3 finalization depends on tested behavior.

## Testing Strategy
- Unit:
  - Runtime resolver/parser tests for default and override precedence.
  - Container helper unit tests (naming, arg building, output parsing, error mapping).
- Integration:
  - Docker-backed command integration for create/ls/exec/rm/reset/auth/tunnel with stubs for nondeterministic edges.
  - Dual-runtime regression tests to ensure host backend remains operational.
- E2E/manual:
  - Default runtime smoke: `create -> ls -> exec -> rm` on a clean Docker host.
  - Host fallback smoke: same flow using `--runtime host` and no Docker dependency.
  - Tunnel/auth spot checks in both runtimes with documented prerequisites.

## Risks & gotchas
- Defaulting to `container` can break users on hosts without Docker. Mitigation: explicit fallback messaging, troubleshooting updates, and easy override (`--runtime host` / env).
- Name collisions between host workspace names and container workspace names can cause destructive targeting mistakes. Mitigation: runtime-scoped resolution + unambiguous errors.
- Porting shell behavior from `agent-env/bin/agent-workspace` to Rust may drift on edge cases (`--no-clone`, tunnel detach logs, auth setup). Mitigation: parity tests + incremental rollout.
- Documentation drift risk is high because current docs are host-native-first. Mitigation: dedicated docs sprint and grep-based consistency checks.
- Security footguns remain for Docker socket and token handling. Mitigation: keep security notes prominent in install/troubleshooting and release docs.

## Rollback plan
- Keep host backend code path intact behind explicit runtime routing throughout rollout.
- If container default causes unacceptable regressions, flip default to `host` via one controlled change in runtime resolver, release patch version, and document temporary fallback.
- If specific container commands regress, gate those commands to host runtime with clear error until patched, rather than disabling whole CLI.
- Revert docs and release guidance to previous host-default wording only if runtime default rollback is executed, keeping one-source-of-truth consistency.
- Preserve `awl_docker` wrapper as emergency compatibility path until dual-runtime cutover is proven stable in release validation.
