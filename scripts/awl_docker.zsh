#!/usr/bin/env zsh

awl_docker() {
  local state_root image agent_env_image
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

  if (( ${+AWL_DOCKER_ARGS} )); then
    if [[ ${(t)AWL_DOCKER_ARGS} == *array* ]]; then
      extra_args=("${AWL_DOCKER_ARGS[@]}")
    elif [[ -n "${AWL_DOCKER_ARGS}" ]]; then
      extra_args=(${=AWL_DOCKER_ARGS})
    fi
  fi
  docker_args+=("${extra_args[@]}")

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
  shift

  if (( ! ${+commands[agent-workspace-launcher]} )); then
    return 0
  fi

  command_argv=(
    agent-workspace-launcher
    __complete
    --shell zsh
    --format describe
    --cword "${cword}"
  )
  for token in "$@"; do
    command_argv+=(--word "${token}")
  done

  AGENT_WORKSPACE_RUNTIME=container command "${command_argv[@]}" 2>/dev/null
}

_awl_docker_completion_legacy() {
  local cur
  local subcmd
  local candidate
  local -a workspace_names candidates filtered

  cur="${words[CURRENT]:-}"
  subcmd="${words[2]:-}"
  workspace_names=(${(f)"$(_awl_docker_workspace_names)"})
  candidates=()
  filtered=()

  if (( CURRENT == 2 )); then
    candidates=(auth create ls rm exec reset tunnel --help --version -h -V)
  else
    case "${subcmd}" in
      auth)
        if (( CURRENT == 3 )); then
          candidates=(github codex gpg --help -h)
        elif (( CURRENT >= 4 )); then
          candidates=("${workspace_names[@]}")
        fi
        ;;
      reset)
        if (( CURRENT == 3 )); then
          candidates=(repo work-repos opt-repos private-repo --help -h)
        elif (( CURRENT == 4 )); then
          candidates=("${workspace_names[@]}")
        fi
        ;;
      rm)
        if (( CURRENT == 3 )); then
          candidates=("${workspace_names[@]}" --all --yes)
        fi
        ;;
      exec|tunnel)
        if (( CURRENT == 3 )); then
          candidates=("${workspace_names[@]}")
        fi
        ;;
    esac
  fi

  for candidate in "${candidates[@]}"; do
    if [[ "${candidate}" == "${cur}"* ]]; then
      filtered+=("${candidate}")
    fi
  done

  if (( ${#filtered[@]} == 0 )); then
    return 1
  fi

  compadd -Q -- "${filtered[@]}"
  return 0
}

_awl_docker_completion_modern() {
  local -i cword
  local line value description
  local -a completion_words raw described described_values plain

  cword=$(( CURRENT - 1 ))
  completion_words=("${words[@]}")
  if (( CURRENT > ${#words[@]} )); then
    completion_words+=("")
  fi

  raw=(${(f)"$(_awl_docker_query_complete "${cword}" "${completion_words[@]}")"})
  if (( ${#raw[@]} == 0 )); then
    return 1
  fi

  described=()
  described_values=()
  plain=()
  for line in "${raw[@]}"; do
    if [[ "${line}" == *$'\t'* ]]; then
      value="${line%%$'\t'*}"
      description="${line#*$'\t'}"
      described+=("${value}:${description}")
      described_values+=("${value}")
    else
      plain+=("${line}")
    fi
  done

  if (( ${#described[@]} > 0 )); then
    if (( ${+functions[_describe]} )); then
      _describe -t awl-docker-candidates "candidate" described
    else
      compadd -Q -- "${described_values[@]}"
    fi
    if (( ${#plain[@]} > 0 )); then
      compadd -Q -- "${plain[@]}"
    fi
    return 0
  fi

  compadd -Q -- "${plain[@]}"
  return 0
}

_awl_docker_completion() {
  if _awl_docker_completion_mode_is_legacy; then
    _awl_docker_completion_legacy
    return $?
  fi

  _awl_docker_completion_modern
}

if (( ${+functions[compdef]} )); then
  compdef _awl_docker_completion awl_docker
fi
