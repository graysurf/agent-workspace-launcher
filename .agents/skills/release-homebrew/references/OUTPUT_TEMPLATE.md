# Release Content

[paste the release notes here]

## 🔗 Links

- Release tag: `vX.Y.Z`
- `release-brew.yml` run URL: [paste URL]
- GitHub release URL: [paste URL]

## Checks

- Required repo checks (`DEVELOPMENT.md`): pass
- `./scripts/release_audit.sh --version vX.Y.Z --branch main --strict`: pass
- Asset payload contract (`bin/agent-workspace-launcher`, `bin/awl`): pass
- `.agents/skills/release-homebrew/scripts/verify-brew-installed-version.sh --version vX.Y.Z`: pass
