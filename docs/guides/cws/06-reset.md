# `cws reset`

Resets repos inside an existing workspace container.

The exact reset behavior is implemented by upstream scripts (`zsh-kit` + `codex-kit`), but the user-facing
interface is stable.

If your workspace contains private repos, `cws reset` may need GitHub auth. When `GH_TOKEN`/`GITHUB_TOKEN` are not
set, the wrapper will automatically reuse your host `gh` login token (keyring) when available (also used by
`cws auth github`).

## Reset a specific repo directory

```sh
cws reset repo <name|container> /work/OWNER/REPO --yes
```

## Reset all work repos

```sh
cws reset work-repos <name|container> --yes
```

## Reset optional repos

```sh
cws reset opt-repos <name|container> --yes
```

## Reset private repo

```sh
cws reset private-repo <name|container> --yes
```
