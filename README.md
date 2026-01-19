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

Common commands:

```sh
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest --help
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest ls
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest exec <name|container>
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest rm <name|container> --yes
docker run --rm -it -v /var/run/docker.sock:/var/run/docker.sock graysurf/codex-workspace-launcher:latest rm --all --yes
```

Docker-outside-of-Docker (DooD) rules:

- The launcher container talks to the host Docker daemon via `-v /var/run/docker.sock:/var/run/docker.sock`.
- Any `-v <src>:<dst>` executed by the launcher resolves `<src>` on the host, not in the launcher container.
- For host file reads (e.g. config snapshot), use absolute host paths and prefer same-path binds + `HOME` passthrough:

```sh
docker run --rm -it \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e HOME="$HOME" \
  -v "$HOME/.config:$HOME/.config:ro" \
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
