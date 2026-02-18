#!/usr/bin/env bash

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  set -euo pipefail
fi

awl_docker() {
  local state_root
  local image
  local agent_env_image
  local -a docker_args extra_args

  state_root="${AWL_DOCKER_STATE:-$HOME/.awl-docker}"
  image="${AWL_DOCKER_IMAGE:-graysurf/agent-workspace-launcher:latest}"
  agent_env_image="${AWL_DOCKER_AGENT_ENV_IMAGE:-graysurf/agent-env:latest}"

  mkdir -p "${state_root}/home" "${state_root}/xdg-state"

  docker_args=(
    --rm
    -it
    -v /var/run/docker.sock:/var/run/docker.sock
    -v "${state_root}:/state"
    -e HOME=/state/home
    -e XDG_STATE_HOME=/state/xdg-state
    -e AGENT_ENV_IMAGE="${agent_env_image}"
  )

  if [[ -n "${AWL_DOCKER_ARGS:-}" ]]; then
    # shellcheck disable=SC2206
    extra_args=(${AWL_DOCKER_ARGS})
    docker_args+=("${extra_args[@]}")
  fi

  docker run "${docker_args[@]}" "${image}" "$@"
}

_awl_docker_workspace_names() {
  awl_docker ls 2>/dev/null | awk '{print $1}'
}

_awl_docker_completion_mode_is_legacy() {
  [[ "${AGENT_WORKSPACE_COMPLETION_MODE:-}" == "legacy" ]]
}

_awl_docker_query_complete() {
  local cword
  local -a command_argv
  local token

  cword="${1:-0}"
  shift || true

  if ! command -v agent-workspace-launcher >/dev/null 2>&1; then
    return 0
  fi

  command_argv=(agent-workspace-launcher __complete --shell bash --cword "${cword}")
  for token in "$@"; do
    command_argv+=(--word "${token}")
  done

  AGENT_WORKSPACE_RUNTIME=container command "${command_argv[@]}" 2>/dev/null
}

_awl_docker_set_compreply() {
  local words
  local cur
  local -a matches

  words="${1:-}"
  cur="${2:-}"
  matches=()

  while IFS= read -r match; do
    matches+=("${match}")
  done < <(compgen -W "${words}" -- "${cur}")

  COMPREPLY=("${matches[@]}")
}

_awl_docker_complete_legacy() {
  local cur
  local subcmd
  local workspaces

  cur="${COMP_WORDS[COMP_CWORD]}"
  subcmd="${COMP_WORDS[1]:-}"
  workspaces="$(_awl_docker_workspace_names)"

  if [[ "${COMP_CWORD}" -eq 1 ]]; then
    _awl_docker_set_compreply "auth create ls rm exec reset tunnel --help --version -h -V" "${cur}"
    return 0
  fi

  case "${subcmd}" in
    auth)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _awl_docker_set_compreply "github codex gpg --help -h" "${cur}"
      elif [[ "${COMP_CWORD}" -ge 3 ]]; then
        _awl_docker_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    reset)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _awl_docker_set_compreply "repo work-repos opt-repos private-repo --help -h" "${cur}"
      elif [[ "${COMP_CWORD}" -eq 3 ]]; then
        _awl_docker_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    rm)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _awl_docker_set_compreply "${workspaces} --all --yes" "${cur}"
      fi
      ;;
    exec|tunnel)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _awl_docker_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    *)
      COMPREPLY=()
      ;;
  esac
}

_awl_docker_complete_modern() {
  local line
  local -a matches

  matches=()
  COMPREPLY=()

  while IFS= read -r line; do
    [[ -n "${line}" ]] || continue
    matches+=("${line}")
  done < <(_awl_docker_query_complete "${COMP_CWORD}" "${COMP_WORDS[@]}")

  COMPREPLY=("${matches[@]}")
}

_awl_docker_complete() {
  if _awl_docker_completion_mode_is_legacy; then
    _awl_docker_complete_legacy
    return 0
  fi

  _awl_docker_complete_modern
}

if type complete >/dev/null 2>&1; then
  complete -o default -F _awl_docker_complete awl_docker
fi

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  awl_docker "$@"
fi
