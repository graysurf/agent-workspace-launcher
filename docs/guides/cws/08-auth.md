# `cws auth`

Updates auth inside an existing workspace container (without recreating it).

Providers:

- `github`: refresh GitHub auth inside the workspace (`gh`/git credentials).
- `codex`: re-apply Codex auth inside the workspace (profile-based, or sync `CODEX_AUTH_FILE`).
- `gpg`: import your signing key so `git commit -S` works in the workspace.

## GitHub auth

Update GitHub auth for an existing workspace (auto-picks the workspace when only one exists):

```sh
cws auth github
```

Update a specific workspace:

```sh
cws auth github <name|container>
```

Use a non-default GitHub hostname:

```sh
cws auth github --host github.com <name|container>
```

Token selection:

- If `GH_TOKEN`/`GITHUB_TOKEN` are set on the host, `cws` forwards them into the launcher container.
- If they are not set and `CWS_AUTH=auto`, `cws` will try to reuse your host `gh` keyring token for `create/reset/auth github`.

## Codex auth

Apply a Codex profile to a workspace:

```sh
cws auth codex --profile work <name|container>
```

Notes:

- Profile-based auth typically requires your host Codex secrets to be available to the launcher container
  (DooD same-path bind). Example:

  ```sh
  CWS_DOCKER_ARGS=(
    -e HOME="$HOME"
    -v "$HOME/.config/codex_secrets:$HOME/.config/codex_secrets:rw"
  )
  cws auth codex --profile work <name|container>
  ```

## GPG auth

Import a GPG signing key into an existing workspace:

```sh
cws auth gpg --key <keyid|fingerprint> <name|container>
```

If you set `CODEX_WORKSPACE_GPG_KEY` on the host, you can omit `--key`:

```sh
export CODEX_WORKSPACE_GPG_KEY="<keyid|fingerprint>"
cws auth gpg <name|container>
```

Notes:

- `auth gpg` exports your secret key and imports it into the workspace container. Treat the workspace container
  as sensitive and remove it when done.
- The launcher container must be able to read your keyring. On macOS hosts, prefer a DooD-safe same-path bind:

  ```sh
  CWS_DOCKER_ARGS=(
    -e HOME="$HOME"
    -v "$HOME/.gnupg:$HOME/.gnupg:ro"
  )
  cws auth gpg --key <keyid|fingerprint> <name|container>
  ```
