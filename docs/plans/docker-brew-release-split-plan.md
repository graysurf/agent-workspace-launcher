# Plan: Split Docker and Brew Release Pipelines

## Overview
This plan separates release delivery into two independent tracks: container publishing for Docker users and archive publishing for Homebrew users. The current dependency on pushing `docker` branch as the only publish trigger will be removed in favor of tag-based releases (`vX.Y.Z`) with explicit workflows per channel. We will keep one source-of-truth version/tag, but split build outputs and verification so failures in one channel do not silently block or mutate the other.

## Scope
- In scope: Introduce dedicated GitHub Actions workflows for Docker image release and Brew artifact release.
- In scope: Keep Docker publishing support (Docker Hub + GHCR) while removing branch-only trigger dependency.
- In scope: Produce versioned release tarballs + sha256 for Homebrew formula consumption.
- In scope: Update release docs/skills/runbooks to describe channel split and exact operator steps.
- In scope: Define and document `homebrew-tap` update flow for `Formula/agent-workspace-launcher.rb`.
- Out of scope: Rewriting archived progress docs/history.
- Out of scope: Full automation of cross-repo PR creation to `homebrew-tap` (can be Phase 2).

## Assumptions (if any)
1. Release versioning remains semver tags in this repo (`vX.Y.Z`).
2. Docker release and Brew release should share the same git tag but run in separate workflows/jobs.
3. Homebrew installs wrapper assets (`scripts/awl.bash`, `scripts/awl.zsh`) rather than Docker image binaries.
4. First delivery can be semi-automated (generate assets + manual tap PR), then evolve to automated tap PR later.

## Sprint 1: Release Contract and Trigger Redesign
**Goal**: Finalize the release event model and channel ownership before workflow changes.
**Demo/Validation**:
- Command(s): `rg -n "docker branch|publish.yml|release" docs/RELEASE_GUIDE.md docs/runbooks/VERSION_BUMPS.md .agents/skills/release-homebrew/SKILL.md`
- Verify: Docs clearly define one tag-based release with two downstream channels (Docker + Brew).

### Task 1.1: Define split-release contract
- **Location**:
  - `docs/RELEASE_GUIDE.md`
  - `docs/runbooks/VERSION_BUMPS.md`
- **Description**: Replace "push docker branch triggers publish" as the primary release contract with tag-based split channels. Keep a short transition note if `docker` branch remains temporarily supported.
- **Dependencies**:
  - none
- **Complexity**: 3
- **Acceptance criteria**:
  - Release guide describes Docker and Brew as separate channels with explicit trigger rules.
  - Operator runbook includes channel-specific verification checkpoints.
- **Validation**:
  - `rg -n "vX.Y.Z|Docker|Brew|homebrew|tag" docs/RELEASE_GUIDE.md docs/runbooks/VERSION_BUMPS.md`

### Task 1.2: Confirm artifact contract for Brew
- **Location**:
  - `docs/RELEASE_GUIDE.md`
  - `README.md`
- **Description**: Define exact asset naming, target matrix, and install paths for Brew-consumed tarballs (for example `agent-workspace-launcher-1.2.3-aarch64-apple-darwin.tar.gz` plus a matching `.sha256` file).
- **Dependencies**:
  - Task 1.1
- **Complexity**: 4
- **Acceptance criteria**:
  - Asset names are deterministic and include target triples.
  - Brew-facing contents are explicitly documented (wrapper scripts, completions/docs if included).
- **Validation**:
  - `rg -n "tar.gz|sha256|target|homebrew|Formula" docs/RELEASE_GUIDE.md README.md`

### Task 1.3: Update release skill contract (operational)
- **Location**:
  - `.agents/skills/release-homebrew/SKILL.md`
- **Description**: Align skill language with the split pipeline model so release operators stop treating Docker branch push as the only publish path.
- **Dependencies**:
  - Task 1.1
- **Complexity**: 2
- **Acceptance criteria**:
  - Skill workflow includes channel-aware publish steps.
  - Legacy wording about zsh-kit pins and branch-only publish is removed.
- **Validation**:
  - `rg -n "zsh-kit|docker branch|Brew|homebrew|tag" .agents/skills/release-homebrew/SKILL.md`

## Sprint 2: Docker Release Workflow Decoupling
**Goal**: Make Docker publish independently triggerable from release tags and manual dispatch.
**Demo/Validation**:
- Command(s): `gh workflow view release-docker.yml` (or `rg -n "on:|tags:|workflow_dispatch" .github/workflows/release-docker.yml`)
- Verify: Docker workflow can publish without requiring a `docker` branch push.

### Task 2.1: Add dedicated Docker workflow
- **Location**:
  - `.github/workflows/release-docker.yml`
- **Description**: Create a Docker-only workflow that builds and pushes multi-arch images on `v*` tag pushes and optional manual dispatch.
- **Dependencies**:
  - Task 1.1
- **Complexity**: 5
- **Acceptance criteria**:
  - Workflow publishes `latest`, the exact release tag (for example `v1.2.3`), and commit tag (for example `sha-a1b2c3d`).
  - Workflow reads `AGENT_KIT_REF` from `VERSIONS.env` and fails on invalid pin format.
- **Validation**:
  - `rg -n "platforms:|docker/build-push-action|AGENT_KIT_REF|tags:" .github/workflows/release-docker.yml`

### Task 2.2: Transition existing publish workflow safely
- **Location**:
  - `.github/workflows/publish.yml`
- **Description**: Either deprecate `publish.yml` or reduce it to a compatibility shim that reuses the new Docker workflow, avoiding duplicate publishes.
- **Dependencies**:
  - Task 2.1
- **Complexity**: 4
- **Acceptance criteria**:
  - No duplicated Docker publish on the same tag.
  - Backward-compatible path is documented if temporary branch trigger support is retained.
- **Validation**:
  - `rg -n "branches: \[docker\]|workflow_call|workflow_dispatch|tags:" .github/workflows/publish.yml .github/workflows/release-docker.yml`

### Task 2.3: Add Docker release smoke verification
- **Location**:
  - `docs/runbooks/INTEGRATION_TEST.md`
- **Description**: Add channel-specific checks for Docker release run URL, manifest platforms, and tag visibility in Docker Hub/GHCR.
- **Dependencies**:
  - Task 2.1
- **Complexity**: 3
- **Acceptance criteria**:
  - Runbook has explicit pass/fail checklist for Docker channel.
- **Validation**:
  - `rg -n "Docker Hub|GHCR|manifest|workflow" docs/runbooks/INTEGRATION_TEST.md`

## Sprint 3: Brew Artifact Release Workflow
**Goal**: Produce Homebrew-consumable release assets from tags with checksums and release attachment.
**Demo/Validation**:
- Command(s): `gh release view vX.Y.Z` and verify asset names/sha files
- Verify: Required tarballs for supported targets are attached to GitHub Release.

### Task 3.1: Add Brew artifact workflow
- **Location**:
  - `.github/workflows/release-brew.yml`
- **Description**: Build and package release tarballs per target, compute sha256, and upload artifacts.
- **Dependencies**:
  - Task 1.2
- **Complexity**: 6
- **Acceptance criteria**:
  - Workflow generates per-target `tar.gz` + `.sha256` files with deterministic names.
  - Artifacts include expected install payload for formula install block.
- **Validation**:
  - `rg -n "tar|sha256|upload-artifact|download-artifact|action-gh-release" .github/workflows/release-brew.yml`

### Task 3.2: Publish/attach assets to GitHub Release
- **Location**:
  - `.github/workflows/release-brew.yml`
- **Description**: Ensure Brew assets are attached to the same `vX.Y.Z` GitHub Release as Docker notes, without overriding Docker metadata.
- **Dependencies**:
  - Task 3.1
- **Complexity**: 4
- **Acceptance criteria**:
  - `gh release view vX.Y.Z` shows all Brew assets.
  - Re-run behavior is idempotent or clearly fail-fast on duplicate assets.
- **Validation**:
  - `gh release view vX.Y.Z --json assets`

### Task 3.3: Document manual checksum extraction for tap updates
- **Location**:
  - `docs/RELEASE_GUIDE.md`
  - `docs/runbooks/VERSION_BUMPS.md`
- **Description**: Add exact commands to fetch/checksum values and update `homebrew-tap` formula safely.
- **Dependencies**:
  - Task 3.2
- **Complexity**: 3
- **Acceptance criteria**:
  - Operator can complete formula update using documented commands only.
- **Validation**:
  - `rg -n "sha256|Formula/agent-workspace-launcher.rb|homebrew-tap" docs/RELEASE_GUIDE.md docs/runbooks/VERSION_BUMPS.md`

## Sprint 4: Tap Integration and Cutover
**Goal**: Enable reliable Homebrew delivery via `graysurf/homebrew-tap` while preserving Docker release stability.
**Demo/Validation**:
- Command(s): run `homebrew-tap` validation suite from its `DEVELOPMENT.md`
- Verify: `brew install agent-workspace-launcher` succeeds from tap using new release assets.

### Task 4.1: Introduce formula in tap repo
- **Location**:
  - `~/Project/graysurf/homebrew-tap/Formula/agent-workspace-launcher.rb`
- **Description**: Add formula using release asset URLs/sha256 for macOS + Linux targets, with a minimal smoke test (`awl --help`).
- **Dependencies**:
  - Task 3.2
- **Complexity**: 5
- **Acceptance criteria**:
  - Formula passes Ruby syntax/style checks and `brew test`.
- **Validation**:
  - `ruby -c Formula/agent-workspace-launcher.rb`
  - `HOMEBREW_NO_AUTO_UPDATE=1 brew style Formula/agent-workspace-launcher.rb`
  - `HOMEBREW_NO_AUTO_UPDATE=1 brew test agent-workspace-launcher`

### Task 4.2: Add release-to-tap operator checklist
- **Location**:
  - `docs/RELEASE_GUIDE.md`
- **Description**: Add a concise checklist for version bump order: tag -> Brew assets -> tap formula update PR -> merge -> brew install verification.
- **Dependencies**:
  - Task 4.1
- **Complexity**: 2
- **Acceptance criteria**:
  - Checklist is linear and includes rollback path when tap update fails.
- **Validation**:
  - `rg -n "tap|Formula|rollback|brew install" docs/RELEASE_GUIDE.md`

### Task 4.3: Decide automation follow-up (optional)
- **Location**:
  - `docs/plans/docker-brew-release-split-plan.md`
- **Description**: Record optional Phase 2 for automated cross-repo PR creation to `homebrew-tap` (requires PAT/repo dispatch policy).
- **Dependencies**:
  - Task 4.2
- **Complexity**: 3
- **Acceptance criteria**:
  - Security and permissions tradeoffs are documented before automation is implemented.
- **Validation**:
  - `rg -n "Phase 2|automation|PAT|repo dispatch" docs/plans/docker-brew-release-split-plan.md`

## Testing Strategy
- Unit:
  - Validate helper scripts used for packaging/checksum generation via `pytest -m script_smoke` and shellcheck.
- Integration:
  - Run both workflows on a dry-run tag in a staging branch/repo; verify release assets and container manifests.
- E2E/manual:
  - Docker: pull tagged images from Docker Hub/GHCR and run `--help` + one create/list smoke case.
  - Brew: install from tap and run `awl --help`, wrapper source check for bash/zsh.

## Risks & gotchas
- One-tag-two-channels can cause partial release states (Docker succeeded, Brew failed). Mitigation: explicit channel status checklist and retry-safe workflows.
- Asset naming drift breaks formula URLs. Mitigation: freeze naming contract and add validation commands in release guide.
- Duplicate publish events from legacy + new workflows. Mitigation: deprecate or guard legacy workflow with clear condition flags.
- Cross-repo coordination lag (`homebrew-tap` PR delayed) can leave users on stale formula. Mitigation: document expected lag and optional automation Phase 2.

## Rollback plan
- If Docker release workflow fails after migration: temporarily re-enable previous publish path (`publish.yml` legacy trigger) and publish hotfix tag.
- If Brew asset workflow fails: keep Docker release active, skip tap update for that version, and publish corrected asset workflow in patch tag.
- If tap formula update introduces install regressions: revert formula commit in `homebrew-tap` and keep prior stable version as default.

## Phase 2 follow-up (decision)
- Decision: keep tap updates manual in Phase 1 (`homebrew-tap` PR by operator after checksum verification).
- Automation candidate: add a gated cross-repo updater job that opens a tap PR via `repo_dispatch` or GitHub API.
- Security tradeoff: automation requires write scope to `graysurf/homebrew-tap`; use least-privilege PAT or fine-grained token.
- Guardrails:
  - run only for semver tags (`vX.Y.Z`)
  - require checksum verification artifacts from `release-brew.yml`
  - dry-run mode before auto-PR in default branch
