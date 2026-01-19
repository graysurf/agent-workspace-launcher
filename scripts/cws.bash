#!/usr/bin/env bash

cws() {
  local image="${CWS_IMAGE:-graysurf/codex-workspace-launcher:latest}"

  local -a run_args
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
  local decl=""
  if decl="$(declare -p CWS_DOCKER_ARGS 2>/dev/null)"; then
    if [[ "$decl" == "declare -a"* ]]; then
      extra_args=("${CWS_DOCKER_ARGS[@]}")
    elif [[ -n "${CWS_DOCKER_ARGS:-}" ]]; then
      read -r -a extra_args <<<"${CWS_DOCKER_ARGS}"
    fi
  elif [[ -n "${CWS_DOCKER_ARGS:-}" ]]; then
    read -r -a extra_args <<<"${CWS_DOCKER_ARGS}"
  fi

  if (( ${#extra_args[@]} > 0 )); then
    run_args+=("${extra_args[@]}")
  fi

  docker "${run_args[@]}" "$image" "$@"
}

_cws_workspaces() {
  command -v docker >/dev/null 2>&1 || return 0
  docker ps -a --filter label=codex-kit.workspace=1 --format '{{.Names}}' 2>/dev/null || true
}

_cws_complete() {
  local cur prev subcmd
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"
  subcmd="${COMP_WORDS[1]:-}"

  if [[ $COMP_CWORD -eq 1 ]]; then
    COMPREPLY=($(compgen -W "create ls exec rm reset tunnel -h --help" -- "$cur"))
    return 0
  fi

  case "$subcmd" in
    create)
      if [[ "$prev" == "--private-repo" || "$prev" == "--name" ]]; then
        COMPREPLY=()
        return 0
      fi
      COMPREPLY=($(compgen -W "--no-extras --private-repo --no-work-repos --name -h --help" -- "$cur"))
      return 0
      ;;
    ls)
      COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
      return 0
      ;;
    exec)
      if [[ "$prev" == "--user" ]]; then
        COMPREPLY=()
        return 0
      fi

      local i=2
      local container=""
      while [[ $i -lt $COMP_CWORD ]]; do
        local w="${COMP_WORDS[i]}"
        case "$w" in
          --root|-h|--help) ;;
          --user) ((i++)) ;;
          --user=*) ;;
          -*) ;;
          *) container="$w"; break ;;
        esac
        ((i++))
      done

      if [[ -z "$container" ]]; then
        COMPREPLY=($(compgen -W "$(_cws_workspaces)" -- "$cur"))
      else
        COMPREPLY=()
      fi
      return 0
      ;;
    rm)
      local has_all=0
      local w
      for w in "${COMP_WORDS[@]}"; do
        [[ "$w" == "--all" ]] && has_all=1
      done

      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "--all --yes -h --help" -- "$cur"))
      elif [[ $has_all -eq 0 ]]; then
        COMPREPLY=($(compgen -W "$(_cws_workspaces)" -- "$cur"))
      else
        COMPREPLY=()
      fi
      return 0
      ;;
    reset)
      if [[ $COMP_CWORD -eq 2 ]]; then
        COMPREPLY=($(compgen -W "repo work-repos opt-repos private-repo -h --help" -- "$cur"))
        return 0
      fi
      if [[ $COMP_CWORD -eq 3 ]]; then
        COMPREPLY=($(compgen -W "$(_cws_workspaces)" -- "$cur"))
        return 0
      fi
      COMPREPLY=()
      return 0
      ;;
    tunnel)
      if [[ "$prev" == "--name" ]]; then
        COMPREPLY=()
        return 0
      fi

      local i2=2
      local container2=""
      while [[ $i2 -lt $COMP_CWORD ]]; do
        local w2="${COMP_WORDS[i2]}"
        case "$w2" in
          --detach|-h|--help) ;;
          --name) ((i2++)) ;;
          --name=*) ;;
          -*) ;;
          *) container2="$w2"; break ;;
        esac
        ((i2++))
      done

      if [[ -z "$container2" ]]; then
        if [[ "$cur" == -* ]]; then
          COMPREPLY=($(compgen -W "--detach --name -h --help" -- "$cur"))
        else
          COMPREPLY=($(compgen -W "$(_cws_workspaces)" -- "$cur"))
        fi
      else
        COMPREPLY=()
      fi
      return 0
      ;;
  esac

  COMPREPLY=()
  return 0
}

complete -F _cws_complete cws

