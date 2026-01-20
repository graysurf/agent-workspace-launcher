# Without `cws` (direct `docker run`)

`cws` is a wrapper. Everything it does can be expressed as a plain `docker run ...`.

## Basic pattern

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN \
  -e GITHUB_TOKEN \
  graysurf/codex-workspace-launcher:latest \
  --help
```

Create a workspace:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN \
  -e GITHUB_TOKEN \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

Update GitHub auth inside an existing workspace:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN \
  -e GITHUB_TOKEN \
  graysurf/codex-workspace-launcher:latest \
  auth github <name|container>
```

Import a GPG signing key into an existing workspace:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.gnupg:$HOME/.gnupg:ro" \
  -e CODEX_WORKSPACE_GPG_KEY="<keyid|fingerprint>" \
  graysurf/codex-workspace-launcher:latest \
  auth gpg <name|container>
```

If you have `gh` logged in on the host and want a one-off token pass-through (without exporting `GH_TOKEN`):

```sh
GH_TOKEN="$(gh auth token -h github.com)" docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

Same idea for `auth github`:

```sh
GH_TOKEN="$(gh auth token -h github.com)" docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN \
  graysurf/codex-workspace-launcher:latest \
  auth github <name|container>
```

Use GHCR instead:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  ghcr.io/graysurf/codex-workspace-launcher:latest \
  ls
```
