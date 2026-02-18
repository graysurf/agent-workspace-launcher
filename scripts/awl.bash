#!/usr/bin/env bash

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  set -euo pipefail
fi

awl() {
  command agent-workspace-launcher "$@"
}

_awl_workspace_names() {
  if ! command -v agent-workspace-launcher >/dev/null 2>&1; then
    return 0
  fi
  command agent-workspace-launcher ls 2>/dev/null | awk '{print $1}'
}

_awl_completion_mode_is_legacy() {
  [[ "${AGENT_WORKSPACE_COMPLETION_MODE:-}" == "legacy" ]]
}

_awl_query_complete() {
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

_awl_set_compreply() {
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

_awl_complete_legacy() {
  local cur
  local subcmd
  local workspaces

  cur="${COMP_WORDS[COMP_CWORD]}"
  subcmd="${COMP_WORDS[1]:-}"
  workspaces="$(_awl_workspace_names)"

  if [[ "${COMP_CWORD}" -eq 1 ]]; then
    _awl_set_compreply "auth create ls rm exec reset tunnel --runtime --help --version -h -V" "${cur}"
    return 0
  fi

  if [[ "${subcmd}" == "--runtime" ]]; then
    if [[ "${COMP_CWORD}" -eq 2 ]]; then
      _awl_set_compreply "container host" "${cur}"
      return 0
    fi
  fi

  case "${subcmd}" in
    auth)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _awl_set_compreply "github codex gpg --help -h" "${cur}"
      elif [[ "${COMP_CWORD}" -ge 3 ]]; then
        _awl_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    reset)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _awl_set_compreply "repo work-repos opt-repos private-repo --help -h" "${cur}"
      elif [[ "${COMP_CWORD}" -eq 3 ]]; then
        _awl_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    rm)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _awl_set_compreply "${workspaces} --all --yes" "${cur}"
      fi
      ;;
    exec|tunnel)
      if [[ "${COMP_CWORD}" -eq 2 ]]; then
        _awl_set_compreply "${workspaces}" "${cur}"
      fi
      ;;
    *)
      COMPREPLY=()
      ;;
  esac
}

_awl_complete_modern() {
  local line
  local -a matches

  matches=()
  COMPREPLY=()

  while IFS= read -r line; do
    [[ -n "${line}" ]] || continue
    matches+=("${line}")
  done < <(_awl_query_complete "${COMP_CWORD}" "${COMP_WORDS[@]}")

  COMPREPLY=("${matches[@]}")
}

_awl_complete() {
  if _awl_completion_mode_is_legacy; then
    _awl_complete_legacy
    return 0
  fi

  _awl_complete_modern
}

# aw* shorthand aliases
alias aw='awl'
alias awa='awl auth'
alias awac='awl auth codex'
alias awah='awl auth github'
alias awag='awl auth gpg'
alias awc='awl create'
alias awls='awl ls'
alias awe='awl exec'
alias awr='awl reset'
alias awrr='awl reset repo'
alias awrw='awl reset work-repos'
alias awro='awl reset opt-repos'
alias awrp='awl reset private-repo'
alias awm='awl rm'
alias awt='awl tunnel'

if type complete >/dev/null 2>&1; then
  complete -o default -F _awl_complete agent-workspace-launcher
  complete -o default -F _awl_complete awl
  complete -o default -F _awl_complete aw
fi

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  awl "$@"
fi
