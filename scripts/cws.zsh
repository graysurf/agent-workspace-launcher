#!/usr/bin/env zsh

_cws_parse_repo_spec() {
  emulate -L zsh
  setopt pipe_fail

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

  reply=("$host" "$owner_repo")
  return 0
}

_cws_detect_gh_target_for_create() {
  emulate -L zsh
  setopt pipe_fail

  local -a argv=("$@")

  local private_repo=''
  local repo=''

  local -i i=1
  while (( i <= ${#argv[@]} )); do
    local arg="${argv[i]-}"
    case "$arg" in
      -h|--help)
        return 1
        ;;
      --private-repo)
        private_repo="${argv[i+1]-}"
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
  local gh_owner_repo=''
  if [[ -n "$candidate" ]]; then
    if _cws_parse_repo_spec "$candidate" "$gh_host"; then
      gh_host="${reply[1]-$gh_host}"
      gh_owner_repo="${reply[2]-}"
    fi
  fi

  reply=("$gh_host" "$gh_owner_repo")
  return 0
}

_cws_detect_gh_host_for_auth_github() {
  emulate -L zsh
  setopt pipe_fail

  local -a argv=("$@")

  local gh_host="${GITHUB_HOST:-github.com}"

  local -i i=1
  while (( i <= ${#argv[@]} )); do
    local arg="${argv[i]-}"
    case "$arg" in
      -h|--help)
        return 1
        ;;
      --host)
        gh_host="${argv[i+1]-$gh_host}"
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

  reply=("$gh_host")
  return 0
}

_cws_gh_keyring_token_for_repo() {
  emulate -L zsh
  setopt pipe_fail

  local gh_host="${1:-github.com}"
  local gh_owner_repo="${2:-}"

  command -v gh >/dev/null 2>&1 || return 0

  local token=''
  token="$(env -u GH_TOKEN -u GITHUB_TOKEN gh auth token -h "$gh_host" 2>/dev/null || true)"
  [[ -n "$token" ]] || return 0

  if [[ -n "$gh_owner_repo" && "$gh_owner_repo" == */* ]]; then
    if GH_TOKEN="$token" GITHUB_TOKEN="" gh api --hostname "$gh_host" --silent "repos/${gh_owner_repo}" >/dev/null 2>&1; then
      print -r -- "$token"
      return 0
    fi
    return 0
  fi

  print -r -- "$token"
  return 0
}

cws() {
  emulate -L zsh
  setopt pipe_fail

  local image="${CWS_IMAGE:-graysurf/codex-workspace-launcher:latest}"

  local -a run_args=()
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

  local injected_token=''
  local auth_mode="${CWS_AUTH:-auto}"
  local env_token="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
  local subcmd="${1:-}"

  local injected_gpg_key=''
  if [[ -z "${CODEX_WORKSPACE_GPG_KEY:-}" ]]; then
    local -i want_gpg=0
    local -i has_explicit_key=0

    if [[ "$subcmd" == "create" ]]; then
      local gpg_mode="${CODEX_WORKSPACE_GPG:-none}"
      case "$gpg_mode" in
        import|true) want_gpg=1 ;;
      esac

      local -a argv_scan=("$@")
      local -i i=1
      while (( i <= ${#argv_scan[@]} )); do
        local arg="${argv_scan[i]-}"
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
    elif [[ "$subcmd" == "auth" && "${2:-}" == "gpg" ]]; then
      want_gpg=1
      local -a argv_scan=("$@")
      local -i i=1
      while (( i <= ${#argv_scan[@]} )); do
        local arg="${argv_scan[i]-}"
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
        injected_gpg_key="${injected_gpg_key##[[:space:]]#}"
        injected_gpg_key="${injected_gpg_key%%[[:space:]]#}"
      fi
    fi
  fi

  if [[ "$auth_mode" != "none" && "$auth_mode" != "env" ]]; then
    local provider="${2:-}"
    if [[ -z "$env_token" && ( "$subcmd" == "create" || "$subcmd" == "reset" || ( "$subcmd" == "auth" && "$provider" == "github" ) ) ]]; then
      local -i want_help=0
      local a=''
      for a in "$@"; do
        if [[ "$a" == "-h" || "$a" == "--help" ]]; then
          want_help=1
          break
        fi
      done

      if (( want_help == 0 )); then
        local gh_host="${GITHUB_HOST:-github.com}"
        local gh_owner_repo=''
        if [[ "$subcmd" == "create" ]]; then
          _cws_detect_gh_target_for_create "${@:2}" || true
          gh_host="${reply[1]-$gh_host}"
          gh_owner_repo="${reply[2]-}"
        elif [[ "$subcmd" == "auth" && "$provider" == "github" ]]; then
          _cws_detect_gh_host_for_auth_github "${@:3}" || true
          gh_host="${reply[1]-$gh_host}"
        fi

        injected_token="$(_cws_gh_keyring_token_for_repo "$gh_host" "$gh_owner_repo" 2>/dev/null || true)"
      fi
    fi
  fi

  local -a extra_args=()
  if (( ${+CWS_DOCKER_ARGS} )); then
    if [[ "${(t)CWS_DOCKER_ARGS}" == *array* ]]; then
      extra_args=("${CWS_DOCKER_ARGS[@]}")
    else
      extra_args=("${(@z)CWS_DOCKER_ARGS}")
    fi
  fi
  if (( ${#extra_args[@]} > 0 )); then
    run_args+=("${extra_args[@]}")
  fi

  if [[ -n "$injected_token" ]]; then
    if [[ -n "$injected_gpg_key" ]]; then
      GH_TOKEN="$injected_token" GITHUB_TOKEN="" CODEX_WORKSPACE_GPG_KEY="$injected_gpg_key" command docker "${run_args[@]}" "$image" "$@"
    else
      GH_TOKEN="$injected_token" GITHUB_TOKEN="" command docker "${run_args[@]}" "$image" "$@"
    fi
  else
    if [[ -n "$injected_gpg_key" ]]; then
      CODEX_WORKSPACE_GPG_KEY="$injected_gpg_key" command docker "${run_args[@]}" "$image" "$@"
    else
      command docker "${run_args[@]}" "$image" "$@"
    fi
  fi
}

_cws_workspaces() {
  emulate -L zsh
  setopt pipe_fail

  command -v docker >/dev/null 2>&1 || return 1

  local -a names=()
  names=(${(f)"$(docker ps -a --filter label=codex-kit.workspace=1 --format '{{.Names}}' 2>/dev/null || true)"})
  if (( ${#names[@]} == 0 )); then
    return 1
  fi

  _values 'workspace' "${names[@]}"
}

_cws() {
  emulate -L zsh
  setopt pipe_fail

  local -a commands=()
  commands=(
    'auth:Update auth for a workspace'
    'create:Create a workspace container'
    'ls:List workspaces'
    'exec:Exec into a workspace'
    'rm:Remove workspaces'
    'reset:Reset repo(s) inside a workspace'
    'tunnel:Start a VS Code tunnel'
  )

  if (( CURRENT == 2 )); then
    _arguments -C \
      '-h[Show help]' \
      '--help[Show help]' \
      '1:command:(auth create ls exec rm reset tunnel)' \
      && return
    return
  fi

  local cmd="${words[2]-}"
  case "$cmd" in
    auth)
      local provider="${words[3]-}"
      case "$provider" in
        codex)
          _arguments -C \
            '-h[Show help]' \
            '--help[Show help]' \
            '2:provider:(codex github gpg)' \
            '--profile[Codex profile name]:profile:' \
            '--container[Workspace name/container]:workspace:_cws_workspaces' \
            '--name[Workspace name/container]:workspace:_cws_workspaces' \
            '3:workspace:_cws_workspaces' \
            && return
          ;;
        github)
          _arguments -C \
            '-h[Show help]' \
            '--help[Show help]' \
            '2:provider:(codex github gpg)' \
            '--host[GitHub hostname]:host:' \
            '--container[Workspace name/container]:workspace:_cws_workspaces' \
            '--name[Workspace name/container]:workspace:_cws_workspaces' \
            '3:workspace:_cws_workspaces' \
            && return
          ;;
        gpg)
          _arguments -C \
            '-h[Show help]' \
            '--help[Show help]' \
            '2:provider:(codex github gpg)' \
            '--key[GPG keyid or fingerprint]:key:' \
            '--container[Workspace name/container]:workspace:_cws_workspaces' \
            '--name[Workspace name/container]:workspace:_cws_workspaces' \
            '3:workspace:_cws_workspaces' \
            && return
          ;;
        *)
          _arguments -C \
            '-h[Show help]' \
            '--help[Show help]' \
            '2:provider:(codex github gpg)' \
            && return
          ;;
      esac
      ;;
    create)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        '--no-extras[Skip cloning ~/.private and extra repos]' \
        '--private-repo[Seed ~/.private from repo]:repo:' \
        '--no-work-repos[Create workspace without cloning repos (requires --name)]' \
        '--name[Workspace name (with --no-work-repos)]:name:' \
        '*:repo:' \
        && return
      ;;
    ls)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        && return
      ;;
    exec)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        '--root[Exec as root]' \
        '--user[Exec as a specific user]:user:' \
        '2:workspace:_cws_workspaces' \
        '*:command:_command_names -e' \
        && return
      ;;
    rm)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        '--all[Remove all workspaces]' \
        '--yes[Skip confirmation]' \
        '2:workspace:_cws_workspaces' \
        && return
      ;;
    reset)
      _arguments -C \
        '-h[Show help]' \
        '--help[Show help]' \
        '2:reset-command:(repo work-repos opt-repos private-repo)' \
        '3:workspace:_cws_workspaces' \
        '*::args:' \
        && return
      ;;
    tunnel)
      _arguments \
        '-h[Show help]' \
        '--help[Show help]' \
        '--detach[Run tunnel in background]' \
        '--name[Tunnel name (<= 20 chars)]:name:' \
        '2:workspace:_cws_workspaces' \
        && return
      ;;
  esac
}

if (( $+functions[compdef] )); then
  compdef _cws cws
else
  if [[ -o interactive && -t 0 ]]; then
    _cws_register_completion() {
      emulate -L zsh
      setopt pipe_fail

      if (( $+functions[compdef] )); then
        compdef _cws cws

        autoload -Uz add-zsh-hook 2>/dev/null || true
        add-zsh-hook -d precmd _cws_register_completion 2>/dev/null || true
        unfunction _cws_register_completion 2>/dev/null || true
      fi
    }

    autoload -Uz add-zsh-hook 2>/dev/null || true
    add-zsh-hook precmd _cws_register_completion 2>/dev/null || true
  fi
fi
