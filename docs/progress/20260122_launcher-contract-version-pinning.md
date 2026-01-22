# codex-workspace-launcher: Launcher contract alignment + version pinning

| Status | Created | Updated |
| --- | --- | --- |
| DRAFT | 2026-01-22 | 2026-01-22 |

Links:

- PR: TBD
- Upstream:
  - codex-kit launcher contract migration: https://github.com/graysurf/codex-kit/pull/64
  - zsh-kit wrapper call-through migration: https://github.com/graysurf/zsh-kit/pull/58
- Docs:
  - [docs/DESIGN.md](../DESIGN.md)
  - [docs/runbooks/INTEGRATION_TEST.md](../runbooks/INTEGRATION_TEST.md)
- Glossary: [docs/templates/PROGRESS_GLOSSARY.md](../templates/PROGRESS_GLOSSARY.md)

## Addendum

- None

## Goal

- Align `codex-workspace-launcher` behavior with the canonical launcher contract (`codex-kit`), without maintaining duplicate semantics in this repo.
- Introduce `VERSIONS.env` as the single source of truth for pinning upstream `zsh-kit` + `codex-kit` refs (and document how to bump safely).
- Centralize real-Docker E2E verification in this repo; keep upstream repos on fast smoke/stub coverage.

## Acceptance Criteria

- `bin/codex-workspace` contains no workspace lifecycle semantics (no custom `rm --all` logic); behavior matches upstream `zsh-kit` + `codex-kit`.
- CI publish workflow uses pinned refs from `VERSIONS.env` (no dynamic `ls-remote` pinning), and built images expose the pinned refs for traceability (label or file).
- Docs no longer mention deprecated/removed low-level env vars (e.g. `CODEX_SECRET_DIR_HOST`, `CODEX_CONFIG_DIR_HOST`, `CODEX_ZSH_PRIVATE_DIR_HOST`) and instead document the current contract (`--secrets-dir`, `--secrets-mount`, `--keep-volumes`, `capabilities`, `--supports`).
- Required pre-submit checks pass (`DEVELOPMENT.md`) and a minimal E2E set passes with real Docker; evidence is recorded under `$CODEX_HOME/out/`.

## Scope

- In-scope:
  - Remove divergent behavior from this repo’s entrypoint wrapper.
  - Add `VERSIONS.env` + a documented bump workflow (runbook) and update CI publish to use it.
  - Update docs (`README.md`, `docs/DESIGN.md`) to reflect the post-migration contract and remove stale env docs.
  - Update E2E plan/gates as needed to validate the launcher contract in the published image.
- Out-of-scope:
  - Further behavior changes in upstream `zsh-kit` / `codex-kit` beyond what is already merged.
  - Adding new subcommands or changing the public CLI contract.
  - Making Linux host support a hard guarantee (keep it “best-effort” unless explicitly expanded).

## I/O Contract

### Input

- `VERSIONS.env` values (`ZSH_KIT_REF`, `CODEX_KIT_REF`) and/or explicit docker build args.
- CLI usage via `docker run ... graysurf/codex-workspace-launcher:<tag> <subcommand> [args]`.

### Output

- Published launcher images whose behavior is fully defined by the pinned upstream refs.
- Documentation and runbooks describing how to bump pinned refs, verify, and release.

### Intermediate Artifacts

- `VERSIONS.env`
- `docs/runbooks/VERSION_BUMPS.md` (or equivalent)
- Evidence logs under `$CODEX_HOME/out/` (e.g. `codex-workspace-launcher-e2e-*.log`)

## Design / Decisions

### Rationale

- The launcher image should be a packaging layer, not a second implementation: user-facing UX lives in `zsh-kit`, and canonical lifecycle semantics live in `codex-kit`.
- Pinning upstream refs in a repo-owned file makes bumps reviewable, reproducible, and easy to audit.
- Centralizing real-Docker E2E here avoids duplicative, flaky, and slow integration suites across multiple repos.

### Risks / Uncertainties

- Risk: accidental behavior divergence (custom wrapper logic) reappears over time.
  - Mitigation: keep `bin/codex-workspace` minimal and add E2E cases that exercise `rm` semantics and JSON output.
- Risk: incompatible upstream pins (zsh-kit expects new launcher capabilities but CODEX_KIT_REF is stale).
  - Mitigation: use `VERSIONS.env` as an explicit pair; validate with E2E and surface pinned refs in image metadata.
- Risk: publish pipeline drift (docs say `main` but workflow triggers on `docker`, etc.).
  - Mitigation: update docs + runbooks to match the actual publish trigger, and make release steps explicit.

## Steps (Checklist)

Note: Any unchecked checkbox in Step 0–3 must include a Reason (inline `Reason: ...` or a nested `- Reason: ...`) before close-progress-pr can complete. Step 4 is excluded (post-merge / wrap-up).
Note: For intentionally deferred / not-do items in Step 0–3, use `- [ ] ~~like this~~` and include `Reason:`. Unchecked and unstruck items (e.g. `- [ ] foo`) will block close-progress-pr.

- [ ] Step 0: Alignment / prerequisites
  - Work Items:
    - [ ] Confirm the “single source of truth” policy: no duplicated lifecycle semantics in `codex-workspace-launcher`.
    - [ ] Decide the pinning strategy for `VERSIONS.env` (prefer commit SHA; optionally record upstream tags in comments).
    - [ ] Decide versioning policy for this repo (independent semver; bump when pinned refs or wrapper behavior changes).
    - [ ] Decide test ownership split and document it:
      - E2E (real Docker): `codex-workspace-launcher`
      - Smoke/stub/fast tests: `zsh-kit`, `codex-kit`
  - Artifacts:
    - `docs/progress/20260122_launcher-contract-version-pinning.md` (this file)
  - Exit Criteria:
    - [ ] Requirements, scope, and acceptance criteria are aligned: `docs/progress/20260122_launcher-contract-version-pinning.md`
    - [ ] Data flow and I/O contract are defined: `docs/progress/20260122_launcher-contract-version-pinning.md`
    - [ ] Risks and rollback plan are defined: `docs/progress/20260122_launcher-contract-version-pinning.md`
    - [ ] Minimal reproducible verification commands are defined (docker + e2e gates): `docs/runbooks/VERSION_BUMPS.md`
- [ ] Step 1: Minimum viable output (MVP)
  - Work Items:
    - [ ] Add `VERSIONS.env` with pinned `ZSH_KIT_REF` + `CODEX_KIT_REF`.
    - [ ] Update `.github/workflows/publish.yml` to use `VERSIONS.env` as the source of truth.
    - [ ] Add traceability output (label or file) that exposes pinned refs in the built image.
    - [ ] Add a bump runbook: `docs/runbooks/VERSION_BUMPS.md` (how to update pins, verify, and release).
  - Artifacts:
    - `VERSIONS.env`
    - `.github/workflows/publish.yml`
    - `docs/runbooks/VERSION_BUMPS.md`
  - Exit Criteria:
    - [ ] One local build uses pinned refs and exposes them (label or file): `docker build ...`
    - [ ] Publish workflow builds deterministically from `VERSIONS.env` (reviewable diff).
    - [ ] Docs skeleton exists for bump/release procedure: `docs/runbooks/VERSION_BUMPS.md`
- [ ] Step 2: Expansion / integration
  - Work Items:
    - [ ] Remove any custom lifecycle behavior from `bin/codex-workspace` (delegate fully to upstream).
    - [ ] Update `README.md` + `docs/DESIGN.md` to reflect current launcher contract and remove stale env vars.
    - [ ] Update E2E plan cases/gates to include coverage for `rm` semantics and (optionally) JSON output flows.
  - Artifacts:
    - `bin/codex-workspace`
    - `README.md`
    - `docs/DESIGN.md`
    - `tests/e2e/*` (if modified)
  - Exit Criteria:
    - [ ] Behavior matches upstream for `rm` (including `--keep-volumes`) and no repo-local overrides remain.
    - [ ] Docs match actual behavior (flags/env) and are copy/paste-ready.
- [ ] Step 3: Validation / testing
  - Work Items:
    - [ ] Run required pre-submit checks from `DEVELOPMENT.md`.
    - [ ] Run a minimal real-Docker E2E set (opt-in) and capture evidence logs under `$CODEX_HOME/out/`.
  - Artifacts:
    - `out/tests/*` (pytest outputs)
    - `$CODEX_HOME/out/codex-workspace-launcher-e2e-*.log`
  - Exit Criteria:
    - [ ] Validation commands executed with results recorded:
      - `.venv/bin/python -m ruff format --check .`
      - `.venv/bin/python -m ruff check .`
      - `.venv/bin/python -m pytest -m script_smoke`
      - `CWS_E2E=1 ... .venv/bin/python -m pytest -m e2e ...` (minimal case set)
    - [ ] Traceable evidence exists (logs + command lines): `$CODEX_HOME/out/`
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

- `VERSIONS.env`: Upstream pinning source of truth (`zsh-kit` + `codex-kit` refs).
- `Dockerfile`: Builds the launcher image and installs pinned upstream code.
- `bin/codex-workspace`: Entrypoint wrapper (must remain minimal; no custom semantics).
- `.github/workflows/publish.yml`: Multi-arch build and publish workflow (should read `VERSIONS.env`).
- `README.md` / `docs/DESIGN.md`: User-facing and design documentation (must match the current contract).
- `tests/e2e/*`: Real Docker E2E coverage for the integrated launcher image.
