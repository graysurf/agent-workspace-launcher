# agent-workspace-launcher: Portable DooD launcher image

| Status | Created | Updated |
| --- | --- | --- |
| DONE | 2026-01-20 | 2026-01-20 |

Links:

- PR: https://github.com/graysurf/agent-workspace-launcher/pull/2
- Planning PR: [#1](https://github.com/graysurf/agent-workspace-launcher/pull/1)
- Implementation PR: [#2](https://github.com/graysurf/agent-workspace-launcher/pull/2)
- Integration testing PR: [#3](https://github.com/graysurf/agent-workspace-launcher/pull/3)
- Docs: [docs/DESIGN.md](../../DESIGN.md)
- User guide: [docs/guides/README.md](../../guides/README.md)
- Integration checklist: [docs/runbooks/INTEGRATION_TEST.md](../../runbooks/INTEGRATION_TEST.md)
- Dev notes: [docs/runbooks/DEV_NOTES.md](../../runbooks/DEV_NOTES.md)
- Glossary: [docs/templates/PROGRESS_GLOSSARY.md](../../templates/PROGRESS_GLOSSARY.md)

## Addendum

- 2026-01-20: Deferred remaining Step 3 validation evidence to an integration testing PR after merge.
  - ~~Linux host smoke run + evidence log.~~
    - Reason: requires a real Linux host with Docker; will be captured in the integration testing PR.
  - ~~CI publish run URL + published image tags.~~
    - Reason: requires a main-branch run (and Docker Hub secrets); will be recorded in the integration testing PR.
- 2026-01-20: Completed deferred Step 3/4 validation evidence (see `docs/runbooks/INTEGRATION_TEST.md`).
  - Linux exploratory smoke evidence (OrbStack engine): `$AGENT_HOME/out/linux-exploratory-smoke-orbstack-20260120-085812.log`
  - CI publish + Docker Hub tags evidence: `$AGENT_HOME/out/ci-publish-verification-20260120-081548.log`
  - CI publish + GHCR tags evidence: `$AGENT_HOME/out/ghcr-verification-20260120-084948.log`

## Goal

- Publish a portable `graysurf/agent-workspace-launcher` image that exposes `agent-workspace create/ls/rm/exec/reset/tunnel` via `docker run`.
- Document DooD footguns (docker.sock + host-path mounts) and auth paths (`GH_TOKEN`) for macOS hosts.

## Acceptance Criteria

- `docker run --rm -it graysurf/agent-workspace-launcher:latest --help` lists `create/ls/rm/exec/reset/tunnel` and exits `0`.
- With `-v /var/run/docker.sock:/var/run/docker.sock`, `ls`, `create graysurf/agent-kit`, `exec <name>`, and `rm <name> --yes` work end-to-end on a clean macOS host.
- With `-e GH_TOKEN="$GH_TOKEN"`, `create OWNER/PRIVATE_REPO` can clone/pull private repos (no host mounts required by default).
- `README.md` documents Quickstart, DooD rules, env vars (including deprecated), and security notes.
- CI publishes multi-arch (`linux/amd64,linux/arm64`) images with `latest` + `sha-<short>` tags (Docker Hub + GHCR).

## Scope

- In-scope:
  - `Dockerfile` + `bin/agent-workspace` wrapper to run `workspace-launcher.zsh` inside the launcher image.
  - Build args to pin `zsh-kit` and `agent-kit` refs at image build time.
  - GitHub Actions workflow for multi-arch build and publish.
  - Documentation updates in `README.md` (Quickstart, env table, security, DooD rules, examples).
  - Manual smoke commands recorded as evidence (macOS; Linux exploratory).
- Out-of-scope:
  - Modifying upstream behavior in `zsh-kit` / `agent-kit` beyond pinning refs.
  - Guaranteeing Linux host support (only smoke exploration; no support claim in docs).
  - Host-side editor integration (e.g. auto `code --new-window`) beyond documenting limitations.
  - New subcommands beyond the existing `create/ls/rm/exec/reset/tunnel` contract.

## I/O Contract

### Input

- CLI: `docker run ... graysurf/agent-workspace-launcher:<tag> <subcommand> [args]`
- Host mounts/env:
  - `-v /var/run/docker.sock:/var/run/docker.sock`
  - optional `-e GH_TOKEN=...` (private repo cloning)
  - optional same-path host mounts (e.g. `-e HOME="$HOME" -v "$HOME/.config:$HOME/.config:ro"`)

### Output

- Host Docker side effects:
  - Workspace container(s) named with `AGENT_WORKSPACE_PREFIX` (default `codex-ws`)
  - Named volumes for workspace persistence (as created by the low-level launcher)
- CLI output for `ls/exec/rm/reset/tunnel` plus standard exit codes

### Intermediate Artifacts

- Repo files: `Dockerfile`, `bin/agent-workspace`, `.github/workflows/publish.yml`, `README.md`
- Evidence logs: `$AGENT_HOME/out/agent-workspace-launcher-smoke-*.md`

## Design / Decisions

### Rationale

- Use the existing two-layer launcher stack from `zsh-kit` (public `create/...`) and `agent-kit` (low-level `up/...`) to avoid re-implementing orchestration logic.
- Pin `zsh-kit` and `agent-kit` refs at build time to make the launcher image reproducible and reduce runtime network dependency.
- Default `AGENT_WORKSPACE_LAUNCHER` to `/opt/agent-kit/docker/agent-env/bin/agent-workspace` and prefer env-based auth (`GH_TOKEN`) inside the launcher container.

### Open Decisions (Step 0)

- Ref pinning (`ZSH_KIT_REF`, `AGENT_KIT_REF`):
  - a) **Selected**: Dockerfile defaults to `main`; CI pins to commit SHAs for published images.
  - b) Always pin Dockerfile defaults to fixed SHAs/tags (manual bump when upstream changes).
  - c) Vendor scripts into this repo/image (no git clone at build; higher maintenance).
- Default runtime image (`AGENT_ENV_IMAGE`):
  - a) **Selected**: rely on upstream default `graysurf/agent-env:linuxbrew` (document; overridable via env).
  - b) Set `ENV AGENT_ENV_IMAGE=graysurf/agent-env:linuxbrew` in Dockerfile for clarity (still overridable).
  - c) Change default to a different image/tag (specify).
- Publish registry + tags:
  - a) Docker Hub only (`graysurf/agent-workspace-launcher`); on push to `main` publish `latest` + `sha-<short>`.
  - b) **Selected**: Docker Hub + GHCR (`ghcr.io/graysurf/agent-workspace-launcher`) with the same tags.
  - c) Publish only on release tags `v*` (optionally also `latest`).

### Proposed Enhancements (Host Wrapper + Shell Completion)

Context: the README “Quickstart” `cws()` wrapper is a much nicer UX than repeating `docker run ...` manually. We
can ship a host-side wrapper (zsh first, then bash) plus completion scripts so users can adopt it with minimal
copy/paste.

Decisions (confirmed):

- Wrapper form + naming:
  - c) **Selected**: provide both a `cws` shell function (sourceable) and an executable `cws` script; completion targets `cws`.
- Distribution / install UX:
  - a) **Selected**: add files under `scripts/` (e.g. `scripts/cws.zsh`, `scripts/cws.bash`) and document “source this in your shell rc”.
  - b) Add an `install` helper script (copies/symlinks into a conventional completion dir).
- Completion packaging:
  - a) **Selected**: completion is defined inside the sourced wrapper file(s) and registers to `cws`.
- Completion behavior:
  - a) **Selected**: static completion for subcommands/flags + dynamic workspace-name completion by querying host `docker ps` (fast; no image call).
  - b) Static-only completion (simpler, less magic).
- Defaults carried by the wrapper:
  - a) **Selected**: always mount `docker.sock` and forward `GH_TOKEN`/`GITHUB_TOKEN` when set; allow extra docker args via `CWS_DOCKER_ARGS` and image override via `CWS_IMAGE`.
  - b) Keep wrapper minimal (only docker.sock) and document manual token/env forwarding.

### Risks / Uncertainties

- Security: mounting `docker.sock` is root-equivalent on the host; mitigation is explicit documentation and safe defaults.
- DooD host-path resolution: any `-v <src>:<dst>` resolves on the host; mitigation is requiring absolute host paths and same-path binds.
- Token exposure: if tokens are passed into workspace containers, they may be visible via `docker inspect`; mitigation is clear docs and avoiding persistence by default.
- Multi-arch publishing: buildx/QEMU differences and runner availability; mitigation is validating on GitHub Actions and documenting known limitations.

## Steps (Checklist)

Note: Any unchecked checkbox in Step 0–3 must include a Reason (inline `Reason: ...` or a nested `- Reason: ...`) before close-progress-pr can complete. Step 4 is excluded (post-merge / wrap-up).
Note: For intentionally deferred / not-do items in Step 0–3, use `- [ ] ~~like this~~` and include `Reason:`. Unchecked and unstruck items (e.g. `- [ ] foo`) will block close-progress-pr.

- [x] Step 0: Alignment / prerequisites
  - Work Items:
    - [x] Confirm external CLI contract matches `workspace-launcher.zsh` help output and `docs/DESIGN.md`.
    - [x] Decide ref pinning strategy (`ZSH_KIT_REF`, `AGENT_KIT_REF`) and default runtime image (`AGENT_ENV_IMAGE`). (Decision: 1a, 2a)
    - [x] Decide publish target(s) and tag strategy (`latest`, `sha-<short>`, optional semver). (Decision: 3b)
  - Artifacts:
    - `docs/progress/20260120_portable-dood-launcher-image.md` (this file)
    - `docs/DESIGN.md` (external contract + smoke commands)
  - Exit Criteria:
    - [x] Requirements, scope, and acceptance criteria are aligned: `docs/progress/20260120_portable-dood-launcher-image.md`.
    - [x] Data flow and I/O contract are defined (DooD + host mounts + env): `docs/DESIGN.md`.
    - [x] Risks and rollback plan are defined (no DB migrations): rollback = revert published tag / pin refs; risks in this file.
    - [x] Minimal reproducible verification commands are defined:
      - `docker run --rm -it graysurf/agent-workspace-launcher:latest --help`
      - `docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/agent-workspace-launcher:latest ls`
      - `docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/agent-workspace-launcher:latest create graysurf/agent-kit`
- [x] Step 1: Minimum viable output (MVP)
  - Work Items:
    - [x] Implement `Dockerfile` with required tooling and clone/pin `zsh-kit` + `agent-kit` into `/opt/...`.
    - [x] Add `bin/agent-workspace` wrapper that sources `workspace-launcher.zsh` and runs `agent-workspace "$@"`.
    - [x] Add minimal `README.md` quickstart and common commands (`--help`, `ls`, `create`).
  - Artifacts:
    - `Dockerfile`
    - `bin/agent-workspace`
    - `README.md`
  - Exit Criteria:
    - [x] At least one happy path runs end-to-end (evidence: `$AGENT_HOME/out/agent-workspace-launcher-step1-smoke-20260120.md`):
      - `docker build -t agent-workspace-launcher:dev .`
      - `docker run --rm -it agent-workspace-launcher:dev --help`
      - `docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock agent-workspace-launcher:dev create graysurf/agent-kit`
    - [x] Primary outputs are verifiable (workspace containers/volumes exist): `docker ps -a` and `docker volume ls`.
    - [x] Usage docs skeleton exists (TL;DR + common commands + DooD rules): `README.md`.
- [x] Step 2: Expansion / integration
  - Work Items:
    - [x] Document and support optional host mounts (secrets/config snapshot) with same-path binds and `HOME` passthrough.
    - [x] Document full env/flags table (including deprecated) from `docs/DESIGN.md`.
    - [x] Validate private repo flows using `GH_TOKEN` inside the launcher container.
  - Artifacts:
    - `README.md`
    - `$AGENT_HOME/out/agent-workspace-launcher-private-repo.md`
  - Exit Criteria:
    - [x] Common branches are covered (evidence: `$AGENT_HOME/out/agent-workspace-launcher-private-repo.md`):
      - [x] `rm <name> --yes`
      - [x] `exec <name>`
      - [x] `reset work-repos <name> --yes`
      - [ ] ~~rm --all --yes~~
        - Reason: destructive on host; not executed in local smoke.
      - [x] Optional mounts are non-fatal when absent; host mount setup is documented in `README.md`.
    - [x] Compatible with existing naming conventions: `AGENT_WORKSPACE_PREFIX` matches upstream behavior (evidence: `$AGENT_HOME/out/agent-workspace-launcher-private-repo.md`).
    - [x] Required migrations / backfill scripts and documentation exist: None (no DB/migrations in this repo).
- [x] Step 3: Validation / testing
  - Work Items:
    - [x] Run macOS smoke suite (help, ls, create, exec, rm, reset) and capture output (evidence: `docs/runbooks/INTEGRATION_TEST.md`).
    - [x] Run Linux exploratory smoke commands (with `--user 0:0` fallback) and capture output (evidence: `docs/runbooks/INTEGRATION_TEST.md`).
    - [x] Verify security docs mention docker.sock risk and token visibility (`docker inspect`) (see `README.md`).
  - Artifacts:
    - `docs/runbooks/INTEGRATION_TEST.md`
    - Evidence logs under `$AGENT_HOME/out/` (see `docs/runbooks/INTEGRATION_TEST.md`)
  - Exit Criteria:
    - [x] Validation commands executed with results recorded: see `docs/runbooks/INTEGRATION_TEST.md`.
    - [x] Run with real repos and representative samples:
      - public: `graysurf/agent-kit`
      - private: `OWNER/PRIVATE_REPO` (with `GH_TOKEN`) and rerun after any fix
    - [x] Traceable evidence exists: smoke logs + CI run URL + published image tags (see `docs/runbooks/INTEGRATION_TEST.md`).
- [x] Step 4: Release / wrap-up
  - Work Items:
    - [x] Add `.github/workflows/publish.yml` to buildx multi-arch and push tags on main.
    - [x] Document tag semantics (`latest`, `sha-<short>`, optional semver) and release workflow notes (see `README.md`).
    - [x] Provide host wrapper scripts + completion (`cws` for zsh, then bash) and document install/customization.
    - [x] Add local build docs (custom tags + `CWS_IMAGE`) and link from `README.md`: `docs/BUILD.md`.
    - [x] Close out progress file when implementation merges (set to DONE and archive).
  - Artifacts:
    - `.github/workflows/publish.yml`
    - Published image tags and workflow run link
  - Exit Criteria:
    - [x] Versioning and changes recorded: `latest` + `sha-<short>` (optional semver later).
    - [x] Release actions completed: GitHub Actions publishes multi-arch images; record workflow run URL and image tags (see `docs/runbooks/INTEGRATION_TEST.md`).
    - [x] Documentation completed and entry points updated: `README.md`, `docs/BUILD.md`, `docs/progress/README.md`.
    - [x] Cleanup completed: set this file to DONE and move to `docs/progress/archived/` when complete.

## Modules

- `Dockerfile`: Build the launcher image and install required tools/dependencies.
- `bin/agent-workspace`: Container entrypoint wrapper that sources `workspace-launcher.zsh`.
- `.github/workflows/publish.yml`: Multi-arch build and publish workflow.
- `README.md`: User-facing docs (Quickstart, DooD rules, env vars, security notes).
- `docs/DESIGN.md`: Architecture and development reference (source of truth).
