#!/usr/bin/env -S zsh -f

# Bundle manifest for `bin/agent-workspace`.
#
# This file is consumed by `$HOME/.config/zsh/tools/bundle-wrapper.zsh` and is not
# meant to be executed directly.

typeset -a sources=(
  "_features/agent-workspace/alias.zsh"
  "_features/agent-workspace/repo-reset.zsh"
  "_features/agent-workspace/workspace-rm.zsh"
  "_features/agent-workspace/workspace-rsync.zsh"
  "_features/agent-workspace/workspace-launcher.zsh"
)
