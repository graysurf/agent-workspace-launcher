# agent-workspace-launcher: Launcher contract alignment + version pinning

| Status | Created | Updated |
| --- | --- | --- |
| DONE | 2026-01-22 | 2026-01-22 |

Links:

- PR: https://github.com/graysurf/agent-workspace-launcher/pull/6
- Planning PR: https://github.com/graysurf/agent-workspace-launcher/pull/5
- Upstream:
  - agent-kit launcher contract migration: https://github.com/graysurf/agent-kit/pull/64
  - zsh-kit wrapper call-through migration: https://github.com/graysurf/zsh-kit/pull/58
- Docs:
  - [docs/DESIGN.md](../../DESIGN.md)
  - [docs/runbooks/INTEGRATION_TEST.md](../../runbooks/INTEGRATION_TEST.md)
  - [docs/runbooks/VERSION_BUMPS.md](../../runbooks/VERSION_BUMPS.md)
- Glossary: [docs/templates/PROGRESS_GLOSSARY.md](../../templates/PROGRESS_GLOSSARY.md)

## Addendum

- None

## Goal

- Align `agent-workspace-launcher` behavior with the canonical launcher contract (`agent-kit`), without maintaining duplicate semantics in this repo.
- Introduce `VERSIONS.env` as the single source of truth for pinning upstream `zsh-kit` + `agent-kit` refs (and document how to bump safely).
- Centralize real-Docker E2E verification in this repo; keep upstream repos on fast smoke/stub coverage.

## Acceptance Criteria

- `bin/agent-workspace` contains no workspace lifecycle semantics (no custom `rm --all` logic); behavior matches upstream `zsh-kit` + `agent-kit`.
- CI publish workflow uses pinned refs from `VERSIONS.env` (no dynamic `ls-remote` pinning), and built images expose the pinned refs for traceability (label or file).
- Docs no longer mention deprecated/removed low-level env vars (e.g. `AGENT_SECRET_DIR_HOST`, `AGENT_CONFIG_DIR_HOST`, `AGENT_ZSH_PRIVATE_DIR_HOST`) and instead document the current contract (`--secrets-dir`, `--secrets-mount`, `--keep-volumes`, `capabilities`, `--supports`).
- Required pre-submit checks pass (`DEVELOPMENT.md`) and a minimal E2E set passes with real Docker; evidence is recorded under `$AGENT_HOME/out/`.

## Scope

- In-scope:
  - Remove divergent behavior from this repo’s entrypoint wrapper.
  - Add `VERSIONS.env` + a documented bump workflow (runbook) and update CI publish to use it.
  - Update docs (`README.md`, `docs/DESIGN.md`) to reflect the post-migration contract and remove stale env docs.
  - Update E2E plan/gates as needed to validate the launcher contract in the published image.
- Out-of-scope:
  - Further behavior changes in upstream `zsh-kit` / `agent-kit` beyond what is already merged.
  - Adding new subcommands or changing the public CLI contract.
  - Making Linux host support a hard guarantee (keep it “best-effort” unless explicitly expanded).

## I/O Contract

### Input

- `VERSIONS.env` values (`ZSH_KIT_REF`, `AGENT_KIT_REF`) and/or explicit docker build args.
- CLI usage via `docker run ... graysurf/agent-workspace-launcher:<tag> <subcommand> [args]`.

### Output

- Published launcher images whose behavior is fully defined by the pinned upstream refs.
- Documentation and runbooks describing how to bump pinned refs, verify, and release.

### Intermediate Artifacts

- `VERSIONS.env`
- `docs/runbooks/VERSION_BUMPS.md` (or equivalent)
- Evidence logs under `$AGENT_HOME/out/` (e.g. `agent-workspace-launcher-e2e-*.log`)

## Design / Decisions

### Rationale

- The launcher image should be a packaging layer, not a second implementation: user-facing UX lives in `zsh-kit`, and canonical lifecycle semantics live in `agent-kit`.
- Pinning upstream refs in a repo-owned file makes bumps reviewable, reproducible, and easy to audit.
- Centralizing real-Docker E2E here avoids duplicative, flaky, and slow integration suites across multiple repos.

### Risks / Uncertainties

- Risk: accidental behavior divergence (custom wrapper logic) reappears over time.
  - Mitigation: keep `bin/agent-workspace` minimal and add E2E cases that exercise `rm` semantics and JSON output.
- Risk: incompatible upstream pins (zsh-kit expects new launcher capabilities but AGENT_KIT_REF is stale).
  - Mitigation: use `VERSIONS.env` as an explicit pair; validate with E2E and surface pinned refs in image metadata.
- Risk: publish pipeline drift (docs say `main` but workflow triggers on `docker`, etc.).
  - Mitigation: update docs + runbooks to match the actual publish trigger, and make release steps explicit.

## Steps (Checklist)

Note: Any unchecked checkbox in Step 0–3 must include a Reason (inline `Reason: ...` or a nested `- Reason: ...`) before close-progress-pr can complete. Step 4 is excluded (post-merge / wrap-up).
Note: For intentionally deferred / not-do items in Step 0–3, use `- [ ] ~~like this~~` and include `Reason:`. Unchecked and unstruck items (e.g. `- [ ] foo`) will block close-progress-pr.

- [x] Step 0: Alignment / prerequisites
  - Work Items:
    - [x] Confirm the “single source of truth” policy: no duplicated lifecycle semantics in `agent-workspace-launcher`.
    - [x] Decide the pinning strategy for `VERSIONS.env` (prefer commit SHA; optionally record upstream tags in comments).
    - [x] Decide versioning policy for this repo (independent semver; bump when pinned refs or wrapper behavior changes).
    - [x] Decide test ownership split and document it:
      - E2E (real Docker): `agent-workspace-launcher`
      - Smoke/stub/fast tests: `zsh-kit`, `agent-kit`
  - Artifacts:
    - `docs/progress/archived/20260122_launcher-contract-version-pinning.md` (this file)
  - Exit Criteria:
    - [x] Requirements, scope, and acceptance criteria are aligned: `docs/progress/20260122_launcher-contract-version-pinning.md`
    - [x] Data flow and I/O contract are defined: `docs/progress/20260122_launcher-contract-version-pinning.md`
    - [x] Risks and rollback plan are defined: `docs/progress/20260122_launcher-contract-version-pinning.md`
    - [x] Minimal reproducible verification commands are defined (docker + e2e gates): `docs/runbooks/VERSION_BUMPS.md`
- [x] Step 1: Minimum viable output (MVP)
  - Work Items:
    - [x] Add `VERSIONS.env` with pinned `ZSH_KIT_REF` + `AGENT_KIT_REF`.
    - [x] Update `.github/workflows/publish.yml` to use `VERSIONS.env` as the source of truth.
    - [x] Add traceability output (label or file) that exposes pinned refs in the built image.
    - [x] Add a bump runbook: `docs/runbooks/VERSION_BUMPS.md` (how to update pins, verify, and release).
  - Artifacts:
    - `VERSIONS.env`
    - `.github/workflows/publish.yml`
    - `docs/runbooks/VERSION_BUMPS.md`
  - Exit Criteria:
    - [x] One local build uses pinned refs and exposes them (label or file): `docker build ...`
    - [x] Publish workflow builds deterministically from `VERSIONS.env` (reviewable diff).
    - [x] Docs skeleton exists for bump/release procedure: `docs/runbooks/VERSION_BUMPS.md`
- [x] Step 2: Expansion / integration
  - Work Items:
    - [x] Remove any custom lifecycle behavior from `bin/agent-workspace` (delegate fully to upstream).
    - [x] Update `README.md` + `docs/DESIGN.md` to reflect current launcher contract and remove stale env vars.
    - [ ] ~~Update E2E plan cases/gates to include coverage for `rm` semantics and (optionally) JSON output flows.~~ Reason: existing e2e plan already covers `rm`; JSON-specific coverage deferred.
  - Artifacts:
    - `bin/agent-workspace`
    - `README.md`
    - `docs/DESIGN.md`
    - `tests/e2e/*` (if modified)
  - Exit Criteria:
    - [x] Behavior matches upstream for `rm` (including `--keep-volumes`) and no repo-local overrides remain.
    - [x] Docs match actual behavior (flags/env) and are copy/paste-ready.
- [x] Step 3: Validation / testing
  - Work Items:
    - [x] Run required pre-submit checks from `DEVELOPMENT.md`.
    - [x] Run a minimal real-Docker E2E set (opt-in).
  - Artifacts:
    - `out/tests/*` (pytest outputs; includes e2e stdout/stderr records)
  - Exit Criteria:
    - [x] Validation commands executed with results recorded:
      - `.venv/bin/python -m ruff format --check .`
      - `.venv/bin/python -m ruff check .`
      - `.venv/bin/python -m pytest -m script_smoke`
      - `CWS_E2E=1 ... .venv/bin/python -m pytest -m e2e ...` (minimal case set)
    - [x] Traceable evidence exists (logs + command lines): `out/tests/`
- [ ] Step 4: Release / wrap-up
  - Work Items:
    - [ ] Update `CHANGELOG.md` and bump version.
    - [ ] Publish images and record tags + workflow run URL.
    - [ ] Close out the progress file (set to DONE and archive via close-progress-pr).
  - Artifacts:
    - `CHANGELOG.md`
    - Release notes / tags / workflow links
  - Exit Criteria:
    - [ ] Versioning and changes recorded: `<version>`, `CHANGELOG.md`
    - [ ] Release actions completed and verifiable (tags + workflow run URL).
    - [ ] Documentation completed and entry points updated.
    - [ ] Cleanup completed (status DONE + archived progress file).

## Modules

- `VERSIONS.env`: Upstream pinning source of truth (`zsh-kit` + `agent-kit` refs).
- `Dockerfile`: Builds the launcher image and installs pinned upstream code.
- `bin/agent-workspace`: Entrypoint wrapper (must remain minimal; no custom semantics).
- `.github/workflows/publish.yml`: Multi-arch build and publish workflow (should read `VERSIONS.env`).
- `README.md` / `docs/DESIGN.md`: User-facing and design documentation (must match the current contract).
- `tests/e2e/*`: Real Docker E2E coverage for the integrated launcher image.
