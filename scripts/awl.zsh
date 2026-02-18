#!/usr/bin/env zsh

awl() {
  command agent-workspace-launcher "$@"
}

_awl_workspace_names() {
  if (( ! ${+commands[agent-workspace-launcher]} )); then
    return 0
  fi
  command agent-workspace-launcher ls 2>/dev/null | awk '{print $1}'
}

_awl_completion() {
  local state
  local -a subcommands auth_commands reset_commands workspace_names rm_targets

  subcommands=(
    "auth"
    "create"
    "ls"
    "rm"
    "exec"
    "reset"
    "tunnel"
    "--runtime"
    "--help"
    "--version"
    "-h"
    "-V"
  )
  auth_commands=("github" "codex" "gpg" "--help" "-h")
  reset_commands=("repo" "work-repos" "opt-repos" "private-repo" "--help" "-h")
  workspace_names=(${(f)"$(_awl_workspace_names)"})
  rm_targets=("${workspace_names[@]}" "--all" "--yes")

  _arguments -C \
    "1:subcommand:->subcommand" \
    "*::arg:->args"

  case "${state}" in
    subcommand)
      _describe -t awl-subcommands "awl subcommand" subcommands
      return 0
      ;;
    args)
      case "${words[2]}" in
        --runtime)
          if (( CURRENT == 3 )); then
            _describe -t awl-runtime-values "runtime" "container host"
          fi
          ;;
        auth)
          if (( CURRENT == 3 )); then
            _describe -t awl-auth-commands "auth command" auth_commands
          elif (( CURRENT >= 4 )); then
            _describe -t awl-workspaces "workspace" workspace_names
          fi
          ;;
        reset)
          if (( CURRENT == 3 )); then
            _describe -t awl-reset-commands "reset command" reset_commands
          elif (( CURRENT == 4 )); then
            _describe -t awl-workspaces "workspace" workspace_names
          fi
          ;;
        rm)
          if (( CURRENT == 3 )); then
            _describe -t awl-rm-targets "rm target" rm_targets
          fi
          ;;
        exec|tunnel)
          if (( CURRENT == 3 )); then
            _describe -t awl-workspaces "workspace" workspace_names
          fi
          ;;
      esac
      return 0
      ;;
  esac
}

# aw* shorthand aliases
if command -v safe_unalias >/dev/null; then
  safe_unalias \
    aw \
    awa awac awah awag \
    awc awls awe \
    awr awrr awrw awro awrp \
    awm awt
fi

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

if (( ${+functions[compdef]} )); then
  compdef _awl_completion awl
  compdef _awl_completion agent-workspace-launcher
  compdef _awl_completion aw
fi
