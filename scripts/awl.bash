#!/usr/bin/env bash

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  set -euo pipefail
fi

awl() {
  command agent-workspace-launcher "$@"
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

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  awl "$@"
fi
