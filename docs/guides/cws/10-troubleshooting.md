# Troubleshooting

## Docker daemon not running

```sh
docker info
```

## Linux `permission denied` on `/var/run/docker.sock`

Some Linux setups require running the launcher as root or adding the socket group:

```sh
docker run --rm -it \
  --user 0:0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/agent-workspace-launcher:latest \
  ls
```

## Find workspace containers manually

Workspaces are labeled:

```sh
docker ps -a --filter label=agent-kit.workspace=1
```

## If `cws` completion is not working (zsh)

Make sure `compinit` is enabled in your shell config:

```sh
autoload -Uz compinit && compinit
```

## Auth issues (private repos)

If you have `gh` logged in on the host, `cws create/reset/auth github` will automatically reuse that token
(keyring) when `GH_TOKEN`/`GITHUB_TOKEN` are not set.

To re-apply GitHub auth to an existing workspace without recreating it:

```sh
cws auth github <name|container>
```

Or pass `GH_TOKEN`/`GITHUB_TOKEN` on the host:

```sh
export GH_TOKEN=...
cws create OWNER/PRIVATE_REPO
```

If org SSO blocks access, refresh your GitHub CLI auth (host-side):

```sh
gh auth refresh -h github.com -s repo -s read:org
```

## GPG signing issues (`git commit -S`)

If `cws auth gpg` fails:

- Ensure you have a signing key on the host: `gpg --list-secret-keys --keyid-format LONG`
- Pass an explicit key: `cws auth gpg --key <keyid|fingerprint> <name|container>`
- If running via DooD, mount your keyring into the launcher container (same-path bind):

  ```sh
  CWS_DOCKER_ARGS=(
    -e HOME="$HOME"
    -v "$HOME/.gnupg:$HOME/.gnupg:ro"
  )
  ```
