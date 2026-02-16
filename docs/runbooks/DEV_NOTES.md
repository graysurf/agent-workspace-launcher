# Dev notes

This folder contains:

- **Runbooks**: stable, executable checklists (ex: integration testing).
- **Dev notes**: short-lived working notes for ongoing development.

Guidelines:

- Keep entries short and date-stamped.
- Prefer links to PRs, progress files, and docs.
- Do not commit large logs or temporary artifacts; store them under `$AGENT_HOME/out/` and link to the paths.
- `$AGENT_HOME/out/` is for short-term local verification and may be cleaned; for durable evidence, prefer PR
  comments and GitHub Actions run URLs.

## 2026-01-20

- Integration testing runbook: `docs/runbooks/INTEGRATION_TEST.md`
- Evidence logs (stored locally under `$AGENT_HOME/out/`):
  - `macos-quickstart-smoke-20260120-080236.log`
  - `ci-publish-verification-20260120-081548.log`
  - `ghcr-verification-20260120-084948.log`
  - `linux-exploratory-smoke-orbstack-20260120-085812.log`
  - `cws-guide-smoke-20260120-093536.log`
  - `cws-guide-create-20260120-093536.log`
