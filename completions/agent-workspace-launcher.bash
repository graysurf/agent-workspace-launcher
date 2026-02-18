#!/usr/bin/env bash

_agent_workspace_launcher_workspace_names() {
  command agent-workspace-launcher ls 2>/dev/null | awk '{print $1}'
}

_agent_workspace_launcher_completion_mode_is_legacy() {
  [[ "${AGENT_WORKSPACE_COMPLETION_MODE:-}" == "legacy" ]]
}

_agent_workspace_launcher_query_complete() {
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

  command "${command_argv[@]}" 2>/dev/null
}

_agent_workspace_launcher_set_compreply() {
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

_agent_workspace_launcher_complete_legacy() {
  local cur
  local subcmd
  local workspaces

  cur="${COMP_WORDS[COMP_CWORD]}"
  subcmd="${COMP_WORDS[1]:-}"
  workspaces="$(_agent_workspace_launcher_workspace_names)"

  if [[ "${COMP_CWORD}" -eq 1 ]]; then
    _agent_workspace_launcher_set_compreply \
      "auth create ls rm exec reset tunnel --help --version -h -V" \
      "${cur}"
    return 0
  fi

  case "${subcmd}" in
    auth)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _agent_workspace_launcher_set_compreply "github codex gpg --help -h" "${cur}"
      elif [[ "${COMP_CWORD}" -ge 3 ]]; then
        _agent_workspace_launcher_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    reset)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _agent_workspace_launcher_set_compreply \
          "repo work-repos opt-repos private-repo --help -h" \
          "${cur}"
      elif [[ "${COMP_CWORD}" -eq 3 ]]; then
        _agent_workspace_launcher_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    rm)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _agent_workspace_launcher_set_compreply "${workspaces} --all --yes" "${cur}"
      fi
      ;;
    exec|tunnel)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _agent_workspace_launcher_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    *)
      COMPREPLY=()
      ;;
  esac
}

_agent_workspace_launcher_complete_modern() {
  local line
  local -a matches

  matches=()
  COMPREPLY=()

  while IFS= read -r line; do
    [[ -n "${line}" ]] || continue
    matches+=("${line}")
  done < <(_agent_workspace_launcher_query_complete "${COMP_CWORD}" "${COMP_WORDS[@]}")

  COMPREPLY=("${matches[@]}")
}

_agent_workspace_launcher_complete() {
  if _agent_workspace_launcher_completion_mode_is_legacy; then
    _agent_workspace_launcher_complete_legacy
    return 0
  fi

  _agent_workspace_launcher_complete_modern
}

if type complete >/dev/null 2>&1; then
  complete -o default -F _agent_workspace_launcher_complete agent-workspace-launcher
  complete -o default -F _agent_workspace_launcher_complete awl
fi
