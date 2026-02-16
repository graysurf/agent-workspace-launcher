# Release Guide (agent-workspace-launcher)

This repo publishes the launcher image from the `docker` branch (see `.github/workflows/publish.yml`).

## Preconditions

- Run in this repo root.
- Working tree is clean: `git status -sb`
- On the target branch (default: `main`): `git branch --show-current`
- Docker is running: `docker info >/dev/null`
- `gh` authenticated (for GitHub Releases): `gh auth status`
- E2E environment configured (via `direnv`/`.envrc` + `.env`, or equivalent)
  - At minimum, repo-backed cases require `CWS_E2E_PUBLIC_REPO`.
  - Auth-heavy cases require additional secrets/mounts (see `DEVELOPMENT.md`).

## Steps

1. Decide version + date
   - Version: `vX.Y.Z`
   - Date: `YYYY-MM-DD` (default: `date +%Y-%m-%d`)

2. Run mandatory local E2E gate (real Docker; full matrix)

   ```sh
   set -euo pipefail
   set -a; source ./VERSIONS.env; set +a

   direnv exec . ./scripts/bump_versions.sh \
     --zsh-kit-ref "$ZSH_KIT_REF" \
     --agent-kit-ref "$AGENT_KIT_REF" \
     --image-tag cws-launcher:e2e \
     --run-e2e
   ```

   Stop if this fails.

3. Update `CHANGELOG.md` (records upstream pins)
   - Prepare a release entry by moving `## Unreleased` into `## vX.Y.Z - YYYY-MM-DD` and injecting pins from `VERSIONS.env`:
     - `./scripts/release_prepare_changelog.sh --version vX.Y.Z --date YYYY-MM-DD`
   - Edit the new release entry for clarity (ensure there are no placeholder bullets like `- None`).

4. Run required repo checks (per `DEVELOPMENT.md`)
   - `.venv/bin/python -m ruff format --check .`
   - `.venv/bin/python -m ruff check .`
   - `.venv/bin/python -m pytest -m script_smoke`

5. Commit the changelog
   - Suggested message: `chore(release): vX.Y.Z`
   - Use the repo’s Semantic Commit helper (see `AGENTS.md`).

6. Audit (strict)
   - `./scripts/release_audit.sh --version vX.Y.Z --branch main --strict`

7. Tag (and push the tag)
   - Create: `git -c tag.gpgSign=false tag vX.Y.Z`
   - Push: `git push origin vX.Y.Z`

8. Publish the GitHub Release from `CHANGELOG.md`
   - Extract notes:
     - `./scripts/release_notes_from_changelog.sh --version vX.Y.Z --output \"$AGENT_HOME/out/release-notes-vX.Y.Z.md\"`
   - Create:
     - `gh release create vX.Y.Z -F \"$AGENT_HOME/out/release-notes-vX.Y.Z.md\" --title \"vX.Y.Z\"`
   - Verify:
     - `gh release view vX.Y.Z`

9. Publish images (this repo’s publish trigger)
   - Fast-forward `docker` to `main` and push:
     - `git fetch origin`
     - `git checkout docker`
     - `git merge --ff-only origin/main`
     - `git push origin docker`
     - `git checkout main`

10. Verify publish
       - Follow `docs/runbooks/INTEGRATION_TEST.md` and record the workflow run URL + tags evidence.
