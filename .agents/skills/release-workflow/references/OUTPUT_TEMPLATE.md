## Release Content
<paste the release notes here>

## 🔗 Links
- Release tag: `vX.Y.Z`
- CI publish run URL: <paste>
- Image tags: `latest`, `sha-<short>`

## Checks
- Local E2E (real Docker): `direnv exec . ./scripts/bump_versions.sh ... --run-e2e` (pass)
- Repo checks: `ruff` + `pytest -m script_smoke` (pass)
