# DooD rules and host mounts

This project is **Docker-outside-of-Docker (DooD)**:

- You run a launcher container.
- The launcher talks to your host Docker daemon via `/var/run/docker.sock`.
- Workspace containers are created on the host.

## Rule 1: mounting `docker.sock` is root-equivalent

If you can mount `/var/run/docker.sock`, you effectively have root access to the host Docker daemon. Treat the
launcher image as trusted code.

## Rule 2: host paths resolve on the host

Any `-v <src>:<dst>` executed by the launcher resolves `<src>` on the host.

If upstream scripts need to read host files, the launcher container must also be able to `test -d` those paths.
In practice, this means **same-path binds**.

Example:

```sh
CWS_DOCKER_ARGS=(
  -e HOME="$HOME"
  -v "$HOME/.config:$HOME/.config:ro"
)
```

## Rule 3: prefer snapshots for config

By default, the launcher snapshots parts of host `~/.config` into the workspace (copy, not bind-mount). This is
safer than bind-mounting your host config into a long-lived container.

For more details, see:

- [README.md](../../../README.md)
- [docs/DESIGN.md](../../DESIGN.md)
