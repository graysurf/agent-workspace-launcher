#!/usr/bin/env bash

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  set -euo pipefail
fi

_cws_parse_repo_spec() {
  local input="${1:-}"
  local default_host="${2:-github.com}"
  [[ -n "$input" ]] || return 1

  local host="$default_host"
  local owner_repo="$input"

  if [[ "$input" == http://* || "$input" == https://* ]]; then
    local without_scheme="${input#*://}"
    host="${without_scheme%%/*}"
    owner_repo="${without_scheme#*/}"
  elif [[ "$input" == git@*:* ]]; then
    local without_user="${input#git@}"
    host="${without_user%%:*}"
    owner_repo="${input#*:}"
  elif [[ "$input" == ssh://git@*/* ]]; then
    local without_prefix="${input#ssh://git@}"
    host="${without_prefix%%/*}"
    owner_repo="${without_prefix#*/}"
  fi

  owner_repo="${owner_repo%.git}"
  owner_repo="${owner_repo%/}"
  if [[ "$owner_repo" == */*/* ]]; then
    local owner="${owner_repo%%/*}"
    local rest="${owner_repo#*/}"
    local name="${rest%%/*}"
    owner_repo="${owner}/${name}"
  fi
  [[ "$owner_repo" == */* ]] || return 1

  printf '%s %s\n' "$host" "$owner_repo"
  return 0
}

_cws_detect_gh_target_for_create() {
  local private_repo=""
  local repo=""

  local i=0
  while (( i < $# )); do
    local arg="${@:i+1:1}"
    case "$arg" in
      -h|--help)
        return 1
        ;;
      --private-repo)
        private_repo="${@:i+2:1}"
        (( i += 2 ))
        ;;
      --private-repo=*)
        private_repo="${arg#*=}"
        (( i += 1 ))
        ;;
      --name)
        (( i += 2 ))
        ;;
      --name=*)
        (( i += 1 ))
        ;;
      --no-extras|--no-work-repos)
        (( i += 1 ))
        ;;
      --)
        (( i += 1 ))
        break
        ;;
      -*)
        (( i += 1 ))
        ;;
      *)
        repo="$arg"
        break
        ;;
    esac
  done

  local candidate="${repo//[[:space:]]/}"
  if [[ -z "$candidate" ]]; then
    candidate="${private_repo//[[:space:]]/}"
  fi

  local gh_host="github.com"
  local gh_owner_repo=""
  if [[ -n "$candidate" ]]; then
    read -r gh_host gh_owner_repo < <(_cws_parse_repo_spec "$candidate" "$gh_host" 2>/dev/null || printf '%s %s\n' "$gh_host" "")
  fi

  printf '%s %s\n' "$gh_host" "$gh_owner_repo"
  return 0
}

_cws_detect_gh_host_for_auth_github() {
  local gh_host="${GITHUB_HOST:-github.com}"

  local i=0
  while (( i < $# )); do
    local arg="${@:i+1:1}"
    case "$arg" in
      -h|--help)
        return 1
        ;;
      --host)
        gh_host="${@:i+2:1}"
        break
        ;;
      --host=*)
        gh_host="${arg#*=}"
        break
        ;;
      --)
        break
        ;;
    esac
    (( i += 1 ))
  done

  gh_host="${gh_host//[[:space:]]/}"
  [[ -n "$gh_host" ]] || gh_host="${GITHUB_HOST:-github.com}"
  printf '%s\n' "$gh_host"
  return 0
}

_cws_gh_keyring_token_for_repo() {
  local gh_host="${1:-github.com}"
  local gh_owner_repo="${2:-}"

  command -v gh >/dev/null 2>&1 || return 0

  local token=""
  token="$(env -u GH_TOKEN -u GITHUB_TOKEN gh auth token -h "$gh_host" 2>/dev/null || true)"
  [[ -n "$token" ]] || return 0

  if [[ -n "$gh_owner_repo" && "$gh_owner_repo" == */* ]]; then
    if GH_TOKEN="$token" GITHUB_TOKEN="" gh api --hostname "$gh_host" --silent "repos/${gh_owner_repo}" >/dev/null 2>&1; then
      printf '%s\n' "$token"
    fi
    return 0
  fi

  printf '%s\n' "$token"
  return 0
}

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
    -e CODEX_WORKSPACE_OPEN_VSCODE_ENABLED
    -e CODEX_WORKSPACE_GPG
    -e CODEX_WORKSPACE_GPG_KEY
  )

  local injected_token=""
  local auth_mode="${CWS_AUTH:-auto}"
  local env_token="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
  local subcmd="${1:-}"
  local provider="${2:-}"

  local injected_gpg_key=""
  if [[ -z "${CODEX_WORKSPACE_GPG_KEY:-}" ]]; then
    local want_gpg=0
    local has_explicit_key=0

    if [[ "$subcmd" == "create" ]]; then
      local gpg_mode="${CODEX_WORKSPACE_GPG:-none}"
      case "$gpg_mode" in
        import|true) want_gpg=1 ;;
      esac

      local i=1
      while (( i <= $# )); do
        local arg="${@:i:1}"
        case "$arg" in
          --gpg)
            want_gpg=1
            ;;
          --no-gpg)
            want_gpg=0
            ;;
          --gpg-key)
            want_gpg=1
            has_explicit_key=1
            (( i += 1 ))
            ;;
          --gpg-key=*)
            want_gpg=1
            has_explicit_key=1
            ;;
        esac
        (( i += 1 ))
      done
    elif [[ "$subcmd" == "auth" && "$provider" == "gpg" ]]; then
      want_gpg=1
      local i=1
      while (( i <= $# )); do
        local arg="${@:i:1}"
        case "$arg" in
          --key)
            has_explicit_key=1
            (( i += 1 ))
            ;;
          --key=*)
            has_explicit_key=1
            ;;
        esac
        (( i += 1 ))
      done
    fi

    if (( want_gpg == 1 && has_explicit_key == 0 )); then
      if command -v git >/dev/null 2>&1; then
        injected_gpg_key="$(git config --global --get user.signingkey 2>/dev/null || true)"
        injected_gpg_key="${injected_gpg_key#"${injected_gpg_key%%[![:space:]]*}"}"
        injected_gpg_key="${injected_gpg_key%"${injected_gpg_key##*[![:space:]]}"}"
      fi
    fi
  fi

  if [[ "$auth_mode" != "none" && "$auth_mode" != "env" ]]; then
    if [[ -z "$env_token" && ( "$subcmd" == "create" || "$subcmd" == "reset" || ( "$subcmd" == "auth" && "$provider" == "github" ) ) ]]; then
      local want_help=0
      local a=""
      for a in "$@"; do
        if [[ "$a" == "-h" || "$a" == "--help" ]]; then
          want_help=1
          break
        fi
      done

      if (( want_help == 0 )); then
        local gh_host="${GITHUB_HOST:-github.com}"
        local gh_owner_repo=""
        if [[ "$subcmd" == "create" ]]; then
          read -r gh_host gh_owner_repo < <(_cws_detect_gh_target_for_create "${@:2}" 2>/dev/null || printf '%s %s\n' "$gh_host" "")
        elif [[ "$subcmd" == "auth" && "$provider" == "github" ]]; then
          gh_host="$(_cws_detect_gh_host_for_auth_github "${@:3}" 2>/dev/null || printf '%s\n' "$gh_host")"
        fi

        injected_token="$(_cws_gh_keyring_token_for_repo "$gh_host" "$gh_owner_repo" 2>/dev/null || true)"
      fi
    fi
  fi

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

  if [[ -n "$injected_token" ]]; then
    if [[ -n "$injected_gpg_key" ]]; then
      GH_TOKEN="$injected_token" GITHUB_TOKEN="" CODEX_WORKSPACE_GPG_KEY="$injected_gpg_key" docker "${run_args[@]}" "$image" "$@"
    else
      GH_TOKEN="$injected_token" GITHUB_TOKEN="" docker "${run_args[@]}" "$image" "$@"
    fi
  else
    if [[ -n "$injected_gpg_key" ]]; then
      CODEX_WORKSPACE_GPG_KEY="$injected_gpg_key" docker "${run_args[@]}" "$image" "$@"
    else
      docker "${run_args[@]}" "$image" "$@"
    fi
  fi
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
    COMPREPLY=($(compgen -W "auth create ls exec rm reset tunnel -h --help" -- "$cur"))
    return 0
  fi

  case "$subcmd" in
    auth)
      local provider="${COMP_WORDS[2]:-}"

      if [[ $COMP_CWORD -eq 2 ]]; then
        COMPREPLY=($(compgen -W "codex github gpg -h --help" -- "$cur"))
        return 0
      fi

      if [[ "$prev" == "--container" || "$prev" == "--name" ]]; then
        COMPREPLY=($(compgen -W "$(_cws_workspaces)" -- "$cur"))
        return 0
      fi

      if [[ "$provider" == "github" && "$prev" == "--host" ]]; then
        COMPREPLY=()
        return 0
      fi

      if [[ "$provider" == "codex" && "$prev" == "--profile" ]]; then
        COMPREPLY=()
        return 0
      fi

      if [[ "$provider" == "gpg" && "$prev" == "--key" ]]; then
        COMPREPLY=()
        return 0
      fi

      local opts="-h --help"
      if [[ "$provider" == "github" ]]; then
        opts="--host --container --name -h --help"
      elif [[ "$provider" == "codex" ]]; then
        opts="--profile --container --name -h --help"
      elif [[ "$provider" == "gpg" ]]; then
        opts="--key --container --name -h --help"
      fi

      if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "$opts" -- "$cur"))
        return 0
      fi

      local i2=3
      local container2=""
      while [[ $i2 -lt $COMP_CWORD ]]; do
        local w2="${COMP_WORDS[i2]}"
        case "$w2" in
          -h|--help) ;;
          --container|--name|--host|--profile|--key) ((i2++)) ;;
          --container=*|--name=*) container2="${w2#*=}" ;;
          --host=*|--profile=*|--key=*) ;;
          -*) ;;
          *) container2="$w2"; break ;;
        esac
        ((i2++))
      done

      if [[ -z "$container2" ]]; then
        COMPREPLY=($(compgen -W "$(_cws_workspaces)" -- "$cur"))
      else
        COMPREPLY=()
      fi
      return 0
      ;;
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

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  cws "$@"
fi
