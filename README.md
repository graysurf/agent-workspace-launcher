# codex-workspace-launcher

Portable Docker launcher for `zsh-kit`'s `codex-workspace`.

This project packages the full `codex-workspace` CLI (`create/ls/rm/exec/reset/tunnel`) into an image so you can
use it without checking out `zsh-kit` or `codex-kit` locally. It operates in Docker-outside-of-Docker mode by
connecting to the host Docker daemon via `/var/run/docker.sock`.

Quickstart:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/REPO
```

Local build:

```sh
docker build -t codex-workspace-launcher:dev \
  --build-arg ZSH_KIT_REF=main \
  --build-arg CODEX_KIT_REF=main \
  .
```

Private repo:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e GH_TOKEN="$GH_TOKEN" \
  graysurf/codex-workspace-launcher:latest \
  create OWNER/PRIVATE_REPO
```

Docs:

- `docs/DESIGN.md`
