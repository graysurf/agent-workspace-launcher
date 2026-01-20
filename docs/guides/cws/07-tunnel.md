# `cws tunnel` (VS Code)

Starts a VS Code tunnel for a running workspace container.

## Start a tunnel

```sh
cws tunnel <name|container>
```

## Detach

```sh
cws tunnel <name|container> --detach
```

## Name the tunnel

```sh
cws tunnel <name|container> --name <tunnel_name>
```

Notes:

- Tunnel setup may require interactive authentication depending on your environment.
- If you only want VS Code UI without tunnels, you can attach with Dev Containers instead (see
  [README.md](../../../README.md)).
