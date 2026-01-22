#!/usr/bin/env -S zsh -f

# Bundle manifest for `bin/codex-workspace`.
#
# This file is consumed by `$HOME/.config/zsh/tools/bundle-wrapper.zsh` and is not
# meant to be executed directly.

typeset -a sources=(
  "_features/codex-workspace/alias.zsh"
  "_features/codex-workspace/repo-reset.zsh"
  "_features/codex-workspace/workspace-rm.zsh"
  "_features/codex-workspace/workspace-rsync.zsh"
  "_features/codex-workspace/workspace-launcher.zsh"
)

