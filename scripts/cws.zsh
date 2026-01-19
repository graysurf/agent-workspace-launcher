#!/usr/bin/env zsh

cws() {
  emulate -L zsh
  setopt pipe_fail

  local image="${CWS_IMAGE:-graysurf/codex-workspace-launcher:latest}"

  local -a run_args=()
  run_args=(run --rm)
  if [[ -t 0 && -t 1 ]]; then
    run_args+=(-it)
  elif [[ -t 0 ]]; then
    run_args+=(-i)
  fi

  run_args+=(
    -v /var/run/docker.sock:/var/run/docker.sock
    -e GH_TOKEN
    -e GITHUB_TOKEN
  )

  local -a extra_args=()
  if (( ${+CWS_DOCKER_ARGS} )); then
    if [[ "${(t)CWS_DOCKER_ARGS}" == *array* ]]; then
      extra_args=("${CWS_DOCKER_ARGS[@]}")
    else
      extra_args=("${(@z)CWS_DOCKER_ARGS}")
    fi
  fi
  if (( ${#extra_args[@]} > 0 )); then
    run_args+=("${extra_args[@]}")
  fi

  command docker "${run_args[@]}" "$image" "$@"
}

_cws_workspaces() {
  emulate -L zsh
  setopt pipe_fail

  command -v docker >/dev/null 2>&1 || return 1

  local -a names=()
  names=(${(f)"$(docker ps -a --filter label=codex-kit.workspace=1 --format '{{.Names}}' 2>/dev/null || true)"})
  if (( ${#names[@]} == 0 )); then
    return 1
  fi

  _values 'workspace' "${names[@]}"
}

_cws() {
  emulate -L zsh
  setopt pipe_fail

  local -a commands=()
  commands=(
    'create:Create a workspace container'
    'ls:List workspaces'
    'exec:Exec into a workspace'
    'rm:Remove workspaces'
    'reset:Reset repo(s) inside a workspace'
    'tunnel:Start a VS Code tunnel'
  )

  local state
  _arguments -C \
    '-h[Show help]' \
    '--help[Show help]' \
    '1:command:->command' \
    '*::arg:->args' || return

  case "$state" in
    command)
      _describe -t commands 'cws command' commands
      return
      ;;
    args)
      ;;
    *)
      return
      ;;
  esac

  local cmd="${words[2]-}"
  case "$cmd" in
    create)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        '--no-extras[Skip cloning ~/.private and extra repos]' \
        '--private-repo[Seed ~/.private from repo]:repo:' \
        '--no-work-repos[Create workspace without cloning repos (requires --name)]' \
        '--name[Workspace name (with --no-work-repos)]:name:' \
        '*:repo:' \
        && return
      ;;
    ls)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        && return
      ;;
    exec)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        '--root[Exec as root]' \
        '--user[Exec as a specific user]:user:' \
        '1:workspace:_cws_workspaces' \
        '*:command:_command_names -e' \
        && return
      ;;
    rm)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        '--all[Remove all workspaces]' \
        '--yes[Skip confirmation]' \
        '1:workspace:_cws_workspaces' \
        && return
      ;;
    reset)
      _arguments -C \
        '-h[Show help]' \
        '--help[Show help]' \
        '1:reset-command:(repo work-repos opt-repos private-repo)' \
        '2:workspace:_cws_workspaces' \
        '*::args:' \
        && return
      ;;
    tunnel)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        '--detach[Run tunnel in background]' \
        '--name[Tunnel name (<= 20 chars)]:name:' \
        '1:workspace:_cws_workspaces' \
        && return
      ;;
  esac
}

if (( $+functions[compdef] )); then
  compdef _cws cws
fi

