use std::ffi::OsString;
use std::io::{IsTerminal, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::EXIT_RUNTIME;

use super::{
    PRIMARY_COMMAND_NAME, RepoSpec, command_exists, confirm_or_abort, default_gpg_signing_key,
    json_escape, normalize_workspace_name_for_create, parse_repo_spec, resolve_codex_auth_file,
    resolve_codex_profile_auth_files, slugify_name, trimmed_nonempty, workspace_prefixes,
    workspace_resolution_candidates,
};

const DEFAULT_CONTAINER_IMAGE: &str = "graysurf/agent-env:latest";
const WORKSPACE_LABEL: &str = "agent-kit.workspace=1";
const DEFAULT_REF: &str = "origin/main";
const CODE_TUNNEL_LOG_PATH: &str = "/home/agent/.agents-env/logs/code-tunnel.log";

const RESET_REPO_SCRIPT: &str = r#"
set -euo pipefail

repo_dir="${1:?missing repo_dir}"
ref="${2:-origin/main}"

if [[ ! -e "$repo_dir/.git" ]]; then
  echo "error: not a git repo: $repo_dir" >&2
  exit 1
fi

cd "$repo_dir"

remote="${ref%%/*}"
branch="${ref#*/}"
if [[ "$remote" == "$ref" || -z "$remote" || -z "$branch" ]]; then
  echo "error: invalid ref (expected remote/branch): $ref" >&2
  exit 2
fi

git fetch --prune -- "$remote" >/dev/null 2>&1 || git fetch --prune -- "$remote"

resolved="$remote/$branch"
if ! git show-ref --verify --quiet "refs/remotes/$resolved"; then
  default_ref="$(git symbolic-ref -q --short "refs/remotes/$remote/HEAD" 2>/dev/null || true)"
  if [[ -n "$default_ref" ]] && git show-ref --verify --quiet "refs/remotes/$default_ref"; then
    echo "warn: $resolved not found; using $default_ref (from $remote/HEAD)" >&2
    resolved="$default_ref"
  elif git show-ref --verify --quiet "refs/remotes/$remote/master"; then
    echo "warn: $resolved not found; using $remote/master" >&2
    resolved="$remote/master"
  else
    echo "error: remote branch not found: $resolved" >&2
    exit 1
  fi
fi

target_branch="${resolved#*/}"
echo "+ reset $repo_dir -> $resolved"

if git show-ref --verify --quiet "refs/heads/$target_branch"; then
  git checkout --force "$target_branch" >/dev/null 2>&1 || {
    git clean -fd >/dev/null 2>&1 || true
    git checkout --force "$target_branch"
  }
else
  git checkout --force -B "$target_branch" "$resolved" >/dev/null 2>&1 || {
    git clean -fd >/dev/null 2>&1 || true
    git checkout --force -B "$target_branch" "$resolved"
  }
fi

if command -v git-reset-remote >/dev/null 2>&1; then
  git-reset-remote --ref "$resolved" --no-fetch --clean --yes
else
  git reset --hard "$resolved"
  git clean -fd
  echo "✅ Done. '$target_branch' now matches '$resolved'."
fi
"#;

const LIST_GIT_REPOS_SCRIPT: &str = r#"
set -euo pipefail

root="${1:?missing root}"
depth="${2:?missing depth}"

if ! [[ "$depth" =~ ^[0-9]+$ ]] || [[ "$depth" -le 0 ]]; then
  echo "error: --depth must be a positive integer (got: $depth)" >&2
  exit 2
fi

if [[ ! -d "$root" ]]; then
  exit 0
fi

git_depth=$((depth + 1))
find -L "$root" -maxdepth "$git_depth" -mindepth 2 \( -type d -o -type f \) -name .git -print0 2>/dev/null \
  | while IFS= read -r -d '' git_entry; do
      printf '%s\n' "${git_entry%/.git}"
    done \
  | sort -u
"#;

const CLONE_SCRIPT: &str = r#"
set -euo pipefail
repo_url="$1"
dest="$2"
ref="${3:-}"

if [[ -d "$dest/.git" ]]; then
  exit 0
fi

if [[ -e "$dest" ]]; then
  echo "error: destination exists but is not a git repo: $dest" >&2
  exit 1
fi

mkdir -p "$(dirname "$dest")"

if [[ -n "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ]]; then
  askpass="/tmp/agent-workspace-git-askpass"
  cat >"$askpass" <<EOS
#!/usr/bin/env bash
case "${1-}" in
  *Username*) echo "x-access-token" ;;
  *Password*) echo "${GH_TOKEN:-${GITHUB_TOKEN:-}}" ;;
  *) echo "" ;;
esac
EOS
  chmod 700 "$askpass"
  GIT_TERMINAL_PROMPT=0 GIT_ASKPASS="$askpass" git clone "$repo_url" "$dest"
  rm -f "$askpass"
else
  GIT_TERMINAL_PROMPT=0 git clone "$repo_url" "$dest"
fi

if [[ -n "$ref" ]]; then
  git -C "$dest" checkout "$ref"
fi
"#;

const SYNC_BASELINE_SCRIPT: &str = r#"
set -euo pipefail

home="${HOME:-/home/agent}"

expand_home() {
  local raw="${1:-}"
  if [[ "$raw" == "~" ]]; then
    printf '%s\n' "$home"
    return 0
  fi
  if [[ "$raw" == "~/"* ]]; then
    printf '%s/%s\n' "${home%/}" "${raw#~/}"
    return 0
  fi
  printf '%s\n' "$raw"
}

zsh_dir="$(expand_home "${ZSH_KIT_DIR:-$home/.config/zsh}")"
agent_dir="$(expand_home "${AGENT_KIT_DIR:-$home/.agents}")"

zsh_repo="${AGENT_WORKSPACE_ZSH_KIT_REPO:-https://github.com/graysurf/zsh-kit.git}"
agent_repo="${AGENT_WORKSPACE_AGENT_KIT_REPO:-https://github.com/graysurf/agent-kit.git}"
nils_formula="${AGENT_WORKSPACE_NILS_CLI_FORMULA:-graysurf/tap/nils-cli}"

sync_main() {
  local target_dir="$1"
  local repo_url="$2"
  rm -rf "$target_dir"
  mkdir -p "$(dirname "$target_dir")"
  GIT_TERMINAL_PROMPT=0 git clone --branch main --single-branch "$repo_url" "$target_dir"
}

echo "+ sync zsh-kit -> $zsh_dir (main)"
sync_main "$zsh_dir" "$zsh_repo"

echo "+ sync agent-kit -> $agent_dir (main)"
sync_main "$agent_dir" "$agent_repo"

if command -v brew >/dev/null 2>&1; then
  echo "+ update nils-cli via Homebrew ($nils_formula)"
  HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 brew tap graysurf/tap >/dev/null 2>&1 \
    || HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 brew tap graysurf/tap
  if ! HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 brew upgrade "$nils_formula" >/dev/null 2>&1; then
    HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 brew install "$nils_formula"
  fi
fi
"#;

#[derive(Debug, Default, Clone)]
struct ParsedCreate {
    show_help: bool,
    no_extras: bool,
    no_work_repos: bool,
    private_repo: Option<String>,
    workspace_name: Option<String>,
    primary_repo: Option<String>,
    extra_repos: Vec<String>,
    ignored_options: Vec<String>,
    image: Option<String>,
    no_pull: bool,
    refspec: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct ParsedRm {
    show_help: bool,
    all: bool,
    yes: bool,
    keep_volumes: bool,
    workspace: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct ParsedAuth {
    show_help: bool,
    provider: Option<String>,
    workspace: Option<String>,
    profile: Option<String>,
    host: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct ParsedResetRepo {
    show_help: bool,
    workspace: Option<String>,
    repo_dir: Option<String>,
    yes: bool,
    refspec: String,
}

#[derive(Debug, Clone)]
struct ParsedResetWorkRepos {
    show_help: bool,
    workspace: Option<String>,
    yes: bool,
    depth: u32,
    root: String,
    refspec: String,
}

impl Default for ParsedResetWorkRepos {
    fn default() -> Self {
        Self {
            show_help: false,
            workspace: None,
            yes: false,
            depth: 4,
            root: String::from("/work"),
            refspec: String::from(DEFAULT_REF),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct ParsedResetSimple {
    show_help: bool,
    workspace: Option<String>,
    yes: bool,
    refspec: String,
}

pub(super) fn dispatch(subcommand: &str, args: &[OsString]) -> i32 {
    match subcommand {
        "create" => run_create(args),
        "ls" => run_ls(args),
        "exec" => run_exec(args),
        "rm" => run_rm(args),
        "tunnel" => run_tunnel(args),
        "auth" => run_auth(args),
        "reset" => run_reset(args),
        _ => {
            eprintln!("error: unknown subcommand: {subcommand}");
            EXIT_RUNTIME
        }
    }
}

fn run_create(args: &[OsString]) -> i32 {
    if !ensure_docker_available() {
        return EXIT_RUNTIME;
    }

    let parsed = match parse_create_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_create_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_create_usage();
        return 0;
    }

    if !parsed.ignored_options.is_empty() {
        eprintln!(
            "warn: ignoring unsupported create options in container mode: {}",
            parsed.ignored_options.join(" ")
        );
    }

    let default_host = std::env::var("GITHUB_HOST").unwrap_or_else(|_| String::from("github.com"));

    let primary_spec = if let Some(primary_repo) = parsed.primary_repo.as_deref() {
        match parse_repo_spec(primary_repo, &default_host) {
            Some(spec) => Some(spec),
            None => {
                eprintln!(
                    "error: invalid primary repo (expected OWNER/REPO or URL): {primary_repo}"
                );
                return EXIT_RUNTIME;
            }
        }
    } else {
        None
    };

    let mut workspace_name = parsed
        .workspace_name
        .clone()
        .or_else(|| {
            primary_spec
                .as_ref()
                .map(|spec| format!("ws-{}", slugify_name(&spec.repo)))
        })
        .unwrap_or_else(generate_workspace_name);
    workspace_name = normalize_workspace_name_for_create(&workspace_name);
    if workspace_name.is_empty() {
        workspace_name = generate_workspace_name();
    }

    let container = normalize_container_name(&workspace_name);
    if container_exists(&container) {
        eprintln!("error: workspace already exists: {container}");
        return EXIT_RUNTIME;
    }

    let image = parsed
        .image
        .clone()
        .or_else(|| std::env::var("AGENT_ENV_IMAGE").ok())
        .or_else(|| std::env::var("CODEX_ENV_IMAGE").ok())
        .and_then(|v| trimmed_nonempty(&v))
        .unwrap_or_else(|| String::from(DEFAULT_CONTAINER_IMAGE));

    if let Err(err) = ensure_image(&image, !parsed.no_pull) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    if let Err(err) = create_workspace_container(&container, &image, primary_spec.as_ref()) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    if let Err(err) = sync_container_baseline(&container) {
        eprintln!("error: failed to sync container baseline: {err}");
        return EXIT_RUNTIME;
    }

    let mut repo_path = String::from("/work");

    if !parsed.no_work_repos
        && let Some(spec) = primary_spec.as_ref()
    {
        repo_path = format!("/work/{}/{}", spec.owner, spec.repo);
        if let Err(err) =
            clone_repo_into_container(&container, spec, &repo_path, parsed.refspec.as_deref())
        {
            eprintln!(
                "error: failed to clone primary repo {}: {err}",
                spec.owner_repo
            );
            return EXIT_RUNTIME;
        }
    }

    if !parsed.no_extras {
        if let Some(private_repo_raw) = parsed.private_repo.as_deref() {
            if let Some(spec) = parse_repo_spec(private_repo_raw, &default_host) {
                let destination = format!("/work/private/{}/{}", spec.owner, spec.repo);
                if let Err(err) = clone_repo_into_container(
                    &container,
                    &spec,
                    &destination,
                    parsed.refspec.as_deref(),
                ) {
                    eprintln!(
                        "warn: failed to clone private repo {}: {err}",
                        spec.owner_repo
                    );
                }
            } else {
                eprintln!(
                    "warn: invalid private repo (expected OWNER/REPO or URL): {private_repo_raw}"
                );
            }
        }

        for extra_repo_raw in &parsed.extra_repos {
            if let Some(spec) = parse_repo_spec(extra_repo_raw, &default_host) {
                let destination = format!("/work/{}/{}", spec.owner, spec.repo);
                if let Err(err) = clone_repo_into_container(
                    &container,
                    &spec,
                    &destination,
                    parsed.refspec.as_deref(),
                ) {
                    eprintln!(
                        "warn: failed to clone extra repo {}: {err}",
                        spec.owner_repo
                    );
                }
            } else {
                eprintln!("warn: invalid repo (expected OWNER/REPO or URL): {extra_repo_raw}");
            }
        }
    }

    println!("workspace: {container}");
    println!("path: {repo_path}");
    0
}

fn run_ls(args: &[OsString]) -> i32 {
    if !ensure_docker_available() {
        return EXIT_RUNTIME;
    }

    let parsed = match super::ls::parse_ls_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_ls_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_ls_usage();
        return 0;
    }

    let mut workspaces = match list_workspace_containers() {
        Ok(items) => items,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };
    workspaces.sort();

    if parsed.json {
        let mut out = String::from("{\"runtime\":\"container\",\"workspaces\":[");
        for (idx, name) in workspaces.iter().enumerate() {
            if idx > 0 {
                out.push(',');
            }
            out.push_str(&format!("{{\"name\":\"{}\"}}", json_escape(name)));
        }
        out.push_str("]}");
        println!("{out}");
    } else {
        for workspace in workspaces {
            println!("{workspace}");
        }
    }

    0
}

fn run_exec(args: &[OsString]) -> i32 {
    if !ensure_docker_available() {
        return EXIT_RUNTIME;
    }

    let parsed = match super::exec::parse_exec_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_exec_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_exec_usage();
        return 0;
    }

    let workspace_name = parsed
        .workspace
        .as_ref()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let container = match resolve_container(&workspace_name) {
        Ok(Some(container)) => container,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let mut command = Command::new("docker");
    command.arg("exec");

    if std::io::stdin().is_terminal() {
        command.arg("-i");
    }
    if std::io::stdout().is_terminal() {
        command.arg("-t");
    }

    if let Some(user) = parsed.user.as_ref() {
        command.arg("-u").arg(user);
    }

    command.arg("-w").arg("/work");
    command.arg(&container);

    if parsed.command.is_empty() {
        command.arg("zsh").arg("-l");
    } else {
        command.args(&parsed.command);
    }

    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    match command.status() {
        Ok(status) => status.code().unwrap_or(EXIT_RUNTIME),
        Err(err) => {
            eprintln!("error: failed to run command in {container}: {err}");
            EXIT_RUNTIME
        }
    }
}

fn run_rm(args: &[OsString]) -> i32 {
    if !ensure_docker_available() {
        return EXIT_RUNTIME;
    }

    let parsed = match parse_rm_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_rm_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_rm_usage();
        return 0;
    }

    if parsed.all && parsed.workspace.is_some() {
        eprintln!("error: rm --all does not accept a workspace name");
        print_rm_usage();
        return EXIT_RUNTIME;
    }

    let targets: Vec<String> = if parsed.all {
        match list_workspace_containers() {
            Ok(items) => items,
            Err(err) => {
                eprintln!("error: {err}");
                return EXIT_RUNTIME;
            }
        }
    } else if let Some(workspace_name) = parsed.workspace.as_deref() {
        match resolve_container(workspace_name) {
            Ok(Some(container)) => vec![container],
            Ok(None) => {
                eprintln!("error: workspace not found: {workspace_name}");
                return EXIT_RUNTIME;
            }
            Err(err) => {
                eprintln!("error: {err}");
                return EXIT_RUNTIME;
            }
        }
    } else {
        eprintln!("error: missing workspace name or --all");
        print_rm_usage();
        return EXIT_RUNTIME;
    };

    if targets.is_empty() {
        return 0;
    }

    if !parsed.yes {
        if parsed.all {
            println!("This will remove {} workspace(s):", targets.len());
        } else {
            println!("This will remove workspace:");
        }
        for target in &targets {
            println!("  - {target}");
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    for target in targets {
        if let Err(err) = docker_status(&["rm", "-f", &target]) {
            eprintln!("error: failed to remove workspace {target}: {err}");
            return EXIT_RUNTIME;
        }

        if !parsed.keep_volumes {
            let (work, home, codex) = volume_names(&target);
            let _ = docker_status(&["volume", "rm", &work, &home, &codex]);
        }

        println!("removed: {target}");
    }

    0
}

fn run_tunnel(args: &[OsString]) -> i32 {
    if !ensure_docker_available() {
        return EXIT_RUNTIME;
    }

    let parsed = match super::tunnel::parse_tunnel_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_tunnel_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_tunnel_usage();
        return 0;
    }

    let workspace_name = if let Some(name) = parsed.workspace.as_deref() {
        name
    } else {
        eprintln!("error: missing workspace name");
        print_tunnel_usage();
        return EXIT_RUNTIME;
    };

    let container = match resolve_container(workspace_name) {
        Ok(Some(container)) => container,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    if !docker_exec_success(&container, "command -v code >/dev/null 2>&1", &[]) {
        eprintln!("error: 'code' command not found in container (required for tunnel)");
        return EXIT_RUNTIME;
    }

    let tunnel_name = parsed
        .tunnel_name
        .as_deref()
        .map(sanitize_tunnel_name)
        .unwrap_or_else(|| default_tunnel_name(&container));

    if parsed.detach {
        let script = format!(
            "mkdir -p \"$(dirname \"{}\")\" && : >\"{}\" && code tunnel --accept-server-license-terms --name \"{}\" >\"{}\" 2>&1",
            CODE_TUNNEL_LOG_PATH, CODE_TUNNEL_LOG_PATH, tunnel_name, CODE_TUNNEL_LOG_PATH
        );

        let mut cmd = Command::new("docker");
        cmd.arg("exec")
            .arg("-d")
            .arg(&container)
            .arg("bash")
            .arg("-lc")
            .arg(script);

        match cmd.status() {
            Ok(status) if status.success() => {
                if parsed.output_json {
                    println!(
                        "{{\"workspace\":\"{}\",\"runtime\":\"container\",\"detached\":true,\"tunnel_name\":\"{}\",\"log_path\":\"{}\"}}",
                        json_escape(&container),
                        json_escape(&tunnel_name),
                        json_escape(CODE_TUNNEL_LOG_PATH)
                    );
                } else {
                    println!("tunnel: {container} detached");
                    println!("log: {CODE_TUNNEL_LOG_PATH}");
                }
                return 0;
            }
            Ok(status) => {
                eprintln!(
                    "error: failed to launch detached tunnel (exit {})",
                    status.code().unwrap_or(EXIT_RUNTIME)
                );
                return EXIT_RUNTIME;
            }
            Err(err) => {
                eprintln!("error: failed to launch detached tunnel: {err}");
                return EXIT_RUNTIME;
            }
        }
    }

    let mut cmd = Command::new("docker");
    cmd.arg("exec");
    if std::io::stdin().is_terminal() {
        cmd.arg("-i");
    }
    if std::io::stdout().is_terminal() {
        cmd.arg("-t");
    }
    cmd.arg(&container);
    cmd.arg("code")
        .arg("tunnel")
        .arg("--accept-server-license-terms")
        .arg("--name")
        .arg(&tunnel_name);

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => {
            let code = status.code().unwrap_or(EXIT_RUNTIME);
            if parsed.output_json {
                println!(
                    "{{\"workspace\":\"{}\",\"runtime\":\"container\",\"detached\":false,\"exit_code\":{},\"tunnel_name\":\"{}\"}}",
                    json_escape(&container),
                    code,
                    json_escape(&tunnel_name)
                );
            }
            code
        }
        Err(err) => {
            eprintln!("error: failed to run tunnel command: {err}");
            EXIT_RUNTIME
        }
    }
}

fn run_auth(args: &[OsString]) -> i32 {
    if !ensure_docker_available() {
        return EXIT_RUNTIME;
    }

    let parsed = match parse_auth_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_auth_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help || parsed.provider.is_none() {
        print_auth_usage();
        return 0;
    }

    let container = match resolve_container_for_auth(parsed.workspace.as_deref()) {
        Ok(container) => container,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let provider = parsed
        .provider
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    match provider.as_str() {
        "github" => run_auth_github(&container, parsed.host.as_deref()),
        "codex" => run_auth_codex(&container, parsed.profile.as_deref()),
        "gpg" => run_auth_gpg(&container, parsed.key.as_deref()),
        _ => {
            eprintln!("error: unknown auth provider: {provider}");
            eprintln!("hint: expected: codex|github|gpg");
            EXIT_RUNTIME
        }
    }
}

fn run_reset(args: &[OsString]) -> i32 {
    if !ensure_docker_available() {
        return EXIT_RUNTIME;
    }

    if args.is_empty() {
        print_reset_usage();
        return 0;
    }

    let subcommand = args[0].to_string_lossy();
    if matches!(subcommand.as_ref(), "-h" | "--help") {
        print_reset_usage();
        return 0;
    }

    match subcommand.as_ref() {
        "repo" => run_reset_repo(&args[1..]),
        "work-repos" => run_reset_work_repos(&args[1..]),
        "opt-repos" => run_reset_opt_repos(&args[1..]),
        "private-repo" => run_reset_private_repo(&args[1..]),
        _ => {
            eprintln!("error: unknown reset subcommand: {subcommand}");
            eprintln!("hint: {PRIMARY_COMMAND_NAME} reset --help");
            EXIT_RUNTIME
        }
    }
}

fn run_reset_repo(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_repo_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_reset_repo_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_reset_repo_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace) = parsed.workspace.as_deref() {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_repo_usage();
        return EXIT_RUNTIME;
    };

    let repo_dir = if let Some(repo_dir) = parsed.repo_dir.as_deref() {
        repo_dir
    } else {
        eprintln!("error: missing repo_dir");
        print_reset_repo_usage();
        return EXIT_RUNTIME;
    };

    let container = match resolve_container(workspace_name) {
        Ok(Some(container)) => container,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let target_repo = map_container_repo_path(repo_dir, "/work");
    if !parsed.yes {
        println!("This will reset a repo in workspace: {container}");
        println!("  - {target_repo}");
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    match reset_repo_in_container(&container, &target_repo, &parsed.refspec) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("error: {err}");
            EXIT_RUNTIME
        }
    }
}

fn run_reset_work_repos(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_work_repos_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_reset_work_repos_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_reset_work_repos_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace) = parsed.workspace.as_deref() {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_work_repos_usage();
        return EXIT_RUNTIME;
    };

    let container = match resolve_container(workspace_name) {
        Ok(Some(container)) => container,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let root = map_container_repo_path(&parsed.root, "/work");
    let repos = match list_git_repos_in_container(&container, &root, parsed.depth) {
        Ok(repos) => repos,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if repos.is_empty() {
        eprintln!(
            "warn: no git repos found under {} (depth={}) in {}",
            root, parsed.depth, container
        );
        return 0;
    }

    if !parsed.yes {
        println!(
            "This will reset {} repo(s) inside workspace: {}",
            repos.len(),
            container
        );
        for repo in &repos {
            println!("  - {repo}");
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    let mut failed = 0usize;
    for repo in repos {
        if let Err(err) = reset_repo_in_container(&container, &repo, &parsed.refspec) {
            eprintln!("error: {err}");
            failed += 1;
        }
    }

    if failed > 0 {
        eprintln!("error: failed to reset {failed} repo(s)");
        return EXIT_RUNTIME;
    }

    0
}

fn run_reset_opt_repos(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_simple_args(args, String::from(DEFAULT_REF)) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_reset_opt_repos_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_reset_opt_repos_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace) = parsed.workspace.as_deref() {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_opt_repos_usage();
        return EXIT_RUNTIME;
    };

    let container = match resolve_container(workspace_name) {
        Ok(Some(container)) => container,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let repos = match list_git_repos_in_container(&container, "/opt", 4) {
        Ok(repos) => repos,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if repos.is_empty() {
        eprintln!("warn: no git repos found under /opt");
        return 0;
    }

    if !parsed.yes {
        println!("This will reset /opt-style repos in workspace: {container}");
        for repo in &repos {
            println!("  - {repo}");
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    for repo in repos {
        if let Err(err) = reset_repo_in_container(&container, &repo, &parsed.refspec) {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    }

    0
}

fn run_reset_private_repo(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_simple_args(args, String::from(DEFAULT_REF)) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            print_reset_private_repo_usage();
            return EXIT_RUNTIME;
        }
    };

    if parsed.show_help {
        print_reset_private_repo_usage();
        return 0;
    }

    let workspace_name = if let Some(workspace) = parsed.workspace.as_deref() {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_private_repo_usage();
        return EXIT_RUNTIME;
    };

    let container = match resolve_container(workspace_name) {
        Ok(Some(container)) => container,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let private_repo = match detect_private_repo_dir(&container) {
        Ok(Some(path)) => path,
        Ok(None) => {
            eprintln!("warn: no private git repo found in workspace: {container}");
            eprintln!(
                "hint: seed it with: AGENT_WORKSPACE_PRIVATE_REPO=OWNER/REPO {PRIMARY_COMMAND_NAME} create ..."
            );
            return 0;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if !parsed.yes {
        println!("This will reset private repo in workspace: {container}");
        println!("  - {private_repo}");
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    match reset_repo_in_container(&container, &private_repo, &parsed.refspec) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("error: {err}");
            EXIT_RUNTIME
        }
    }
}

fn parse_create_args(args: &[OsString]) -> Result<ParsedCreate, String> {
    let mut parsed = ParsedCreate::default();
    let mut idx = 0usize;
    let mut positional_only = false;

    while idx < args.len() {
        let current = args[idx].clone();
        let text = current.to_string_lossy().into_owned();

        if !positional_only {
            match text.as_str() {
                "-h" | "--help" => {
                    parsed.show_help = true;
                    idx += 1;
                    continue;
                }
                "--no-work-repos" => {
                    parsed.no_work_repos = true;
                    idx += 1;
                    continue;
                }
                "--no-extras" => {
                    parsed.no_extras = true;
                    idx += 1;
                    continue;
                }
                "--no-pull" => {
                    parsed.no_pull = true;
                    idx += 1;
                    continue;
                }
                "--private-repo" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --private-repo"));
                    }
                    parsed.private_repo = trimmed_nonempty(args[idx].to_string_lossy().as_ref());
                    idx += 1;
                    continue;
                }
                "--name" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --name"));
                    }
                    let value = args[idx].to_string_lossy().into_owned();
                    let normalized = normalize_workspace_name_for_create(&value);
                    parsed.workspace_name = trimmed_nonempty(&normalized);
                    idx += 1;
                    continue;
                }
                "--image" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --image"));
                    }
                    parsed.image = trimmed_nonempty(args[idx].to_string_lossy().as_ref());
                    idx += 1;
                    continue;
                }
                "--ref" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --ref"));
                    }
                    parsed.refspec = trimmed_nonempty(args[idx].to_string_lossy().as_ref());
                    idx += 1;
                    continue;
                }
                "--" => {
                    positional_only = true;
                    idx += 1;
                    continue;
                }
                _ if text.starts_with("--private-repo=") => {
                    parsed.private_repo = trimmed_nonempty(text["--private-repo=".len()..].trim());
                    idx += 1;
                    continue;
                }
                _ if text.starts_with("--name=") => {
                    let value = text["--name=".len()..].trim();
                    let normalized = normalize_workspace_name_for_create(value);
                    parsed.workspace_name = trimmed_nonempty(&normalized);
                    idx += 1;
                    continue;
                }
                _ if text.starts_with("--image=") => {
                    parsed.image = trimmed_nonempty(text["--image=".len()..].trim());
                    idx += 1;
                    continue;
                }
                _ if text.starts_with("--ref=") => {
                    parsed.refspec = trimmed_nonempty(text["--ref=".len()..].trim());
                    idx += 1;
                    continue;
                }
                _ if text.starts_with('-') => {
                    parsed.ignored_options.push(text);
                    idx += 1;
                    continue;
                }
                _ => {}
            }
        }

        if parsed.primary_repo.is_none() {
            parsed.primary_repo = Some(text);
        } else {
            parsed.extra_repos.push(text);
        }
        idx += 1;
    }

    if parsed.no_work_repos && (parsed.primary_repo.is_some() || !parsed.extra_repos.is_empty()) {
        return Err(String::from("--no-work-repos does not accept repo args"));
    }

    Ok(parsed)
}

fn parse_rm_args(args: &[OsString]) -> Result<ParsedRm, String> {
    let mut parsed = ParsedRm {
        keep_volumes: false,
        ..ParsedRm::default()
    };

    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--all" => parsed.all = true,
            "-y" | "--yes" => parsed.yes = true,
            "--keep-volumes" => parsed.keep_volumes = true,
            "--volumes" => parsed.keep_volumes = false,
            _ if text.starts_with('-') => return Err(format!("unknown option for rm: {text}")),
            _ => {
                if parsed.workspace.is_some() {
                    return Err(String::from("rm accepts at most one workspace name"));
                }
                parsed.workspace = Some(text.to_string());
            }
        }
    }

    Ok(parsed)
}

fn parse_auth_args(args: &[OsString]) -> Result<ParsedAuth, String> {
    let mut parsed = ParsedAuth::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--container" | "--workspace" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(format!("missing value for {}", current));
                }
                parsed.workspace = Some(args[idx].to_string_lossy().into_owned());
            }
            "--profile" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --profile"));
                }
                parsed.profile = Some(args[idx].to_string_lossy().into_owned());
            }
            "--host" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --host"));
                }
                parsed.host = Some(args[idx].to_string_lossy().into_owned());
            }
            "--key" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --key"));
                }
                parsed.key = Some(args[idx].to_string_lossy().into_owned());
            }
            _ if current.starts_with("--container=") => {
                parsed.workspace = Some(current["--container=".len()..].to_string());
            }
            _ if current.starts_with("--workspace=") => {
                parsed.workspace = Some(current["--workspace=".len()..].to_string());
            }
            _ if current.starts_with("--profile=") => {
                parsed.profile = Some(current["--profile=".len()..].to_string());
            }
            _ if current.starts_with("--host=") => {
                parsed.host = Some(current["--host=".len()..].to_string());
            }
            _ if current.starts_with("--key=") => {
                parsed.key = Some(current["--key=".len()..].to_string());
            }
            _ if current.starts_with('-') => {
                return Err(format!("unknown option for auth: {current}"));
            }
            _ => {
                let text = current.to_string();
                if parsed.provider.is_none() {
                    parsed.provider = Some(text);
                } else if parsed.workspace.is_none() {
                    parsed.workspace = Some(text);
                } else {
                    return Err(format!("unexpected arg: {current}"));
                }
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

fn parse_reset_repo_args(args: &[OsString]) -> Result<ParsedResetRepo, String> {
    let mut parsed = ParsedResetRepo {
        refspec: String::from(DEFAULT_REF),
        ..ParsedResetRepo::default()
    };
    let mut idx = 0usize;

    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "-y" | "--yes" => parsed.yes = true,
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            _ if current.starts_with("--ref=") => {
                parsed.refspec = current["--ref=".len()..].to_string();
            }
            _ if current.starts_with('-') => {
                return Err(format!("unknown option for reset repo: {current}"));
            }
            _ => {
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(current.to_string());
                } else if parsed.repo_dir.is_none() {
                    parsed.repo_dir = Some(current.to_string());
                } else {
                    return Err(format!("unexpected arg for reset repo: {current}"));
                }
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

fn parse_reset_work_repos_args(args: &[OsString]) -> Result<ParsedResetWorkRepos, String> {
    let mut parsed = ParsedResetWorkRepos::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "-y" | "--yes" => parsed.yes = true,
            "--depth" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --depth"));
                }
                parsed.depth = parse_positive_u32(args[idx].to_string_lossy().as_ref(), "--depth")?;
            }
            "--root" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --root"));
                }
                parsed.root = args[idx].to_string_lossy().into_owned();
            }
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            _ if current.starts_with("--depth=") => {
                parsed.depth = parse_positive_u32(&current["--depth=".len()..], "--depth")?;
            }
            _ if current.starts_with("--root=") => {
                parsed.root = current["--root=".len()..].to_string()
            }
            _ if current.starts_with("--ref=") => {
                parsed.refspec = current["--ref=".len()..].to_string()
            }
            _ if current.starts_with('-') => {
                return Err(format!("unknown option for reset work-repos: {current}"));
            }
            _ => {
                if parsed.workspace.is_some() {
                    return Err(format!("unexpected arg for reset work-repos: {current}"));
                }
                parsed.workspace = Some(current.to_string());
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

fn parse_reset_simple_args(
    args: &[OsString],
    default_ref: String,
) -> Result<ParsedResetSimple, String> {
    let mut parsed = ParsedResetSimple {
        refspec: default_ref,
        ..ParsedResetSimple::default()
    };
    let mut idx = 0usize;

    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "-y" | "--yes" => parsed.yes = true,
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            _ if current.starts_with("--ref=") => {
                parsed.refspec = current["--ref=".len()..].to_string()
            }
            _ if current.starts_with('-') => return Err(format!("unknown option: {current}")),
            _ => {
                if parsed.workspace.is_some() {
                    return Err(format!("unexpected arg: {current}"));
                }
                parsed.workspace = Some(current.to_string());
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

fn parse_positive_u32(raw: &str, option_name: &str) -> Result<u32, String> {
    let value = raw
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("{option_name} must be a positive integer (got: {raw})"))?;
    if value == 0 {
        return Err(format!(
            "{option_name} must be a positive integer (got: {raw})"
        ));
    }
    Ok(value)
}

fn primary_workspace_prefix() -> String {
    workspace_prefixes()
        .into_iter()
        .next()
        .unwrap_or_else(|| String::from("agent-ws"))
}

fn normalize_container_name(name: &str) -> String {
    let prefix = primary_workspace_prefix();
    if name.starts_with(&(prefix.clone() + "-")) {
        return name.to_string();
    }

    let mut normalized = name.to_string();
    if prefix.ends_with("-ws") && normalized.starts_with("ws-") {
        let stripped = normalized.trim_start_matches("ws-");
        if !stripped.is_empty() {
            normalized = stripped.to_string();
        }
    }

    format!("{prefix}-{normalized}")
}

fn generate_workspace_name() -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("ws-{suffix}")
}

fn ensure_docker_available() -> bool {
    if command_exists("docker") {
        true
    } else {
        eprintln!("error: docker command not found in PATH");
        false
    }
}

fn docker_status(args: &[&str]) -> Result<(), String> {
    let status = Command::new("docker")
        .args(args)
        .status()
        .map_err(|err| format!("failed to run docker {}: {err}", args.join(" ")))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "docker {} failed (exit {})",
            args.join(" "),
            status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

fn docker_output(args: &[&str]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .map_err(|err| format!("failed to run docker {}: {err}", args.join(" ")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!(
                "docker {} failed (exit {})",
                args.join(" "),
                output.status.code().unwrap_or(EXIT_RUNTIME)
            ))
        } else {
            Err(stderr)
        }
    }
}

fn docker_exec_success(container: &str, script: &str, extra_args: &[&str]) -> bool {
    let mut cmd = Command::new("docker");
    cmd.arg("exec")
        .arg(container)
        .arg("bash")
        .arg("-lc")
        .arg(script)
        .arg("--");
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn list_workspace_containers() -> Result<Vec<String>, String> {
    let output = docker_output(&[
        "ps",
        "-a",
        "--filter",
        &format!("label={WORKSPACE_LABEL}"),
        "--format",
        "{{.Names}}",
    ])?;

    let mut names: Vec<String> = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    names.sort();
    Ok(names)
}

pub(super) fn completion_workspace_names() -> Result<Vec<String>, String> {
    list_workspace_containers()
}

fn container_exists(name: &str) -> bool {
    Command::new("docker")
        .args(["inspect", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn container_running(name: &str) -> bool {
    if let Ok(output) = docker_output(&["inspect", "-f", "{{.State.Running}}", name]) {
        return output.trim() == "true";
    }
    false
}

fn ensure_container_running(name: &str) -> Result<(), String> {
    if !container_exists(name) {
        return Err(format!("workspace not found: {name}"));
    }

    if container_running(name) {
        return Ok(());
    }

    docker_status(&["start", name])
}

fn resolve_container(name: &str) -> Result<Option<String>, String> {
    let cleaned = match trimmed_nonempty(name) {
        Some(name) => name,
        None => return Ok(None),
    };

    if container_exists(&cleaned) {
        return Ok(Some(cleaned));
    }

    let prefixes = workspace_prefixes();
    for candidate in workspace_resolution_candidates(&cleaned, &prefixes) {
        if container_exists(&candidate) {
            return Ok(Some(candidate));
        }

        let normalized = normalize_container_name(&candidate);
        if container_exists(&normalized) {
            return Ok(Some(normalized));
        }
    }

    Ok(None)
}

fn resolve_container_for_auth(name: Option<&str>) -> Result<String, String> {
    if let Some(name) = name.and_then(trimmed_nonempty) {
        return match resolve_container(&name)? {
            Some(container) => Ok(container),
            None => Err(format!("workspace not found: {name}")),
        };
    }

    let workspaces = list_workspace_containers()?;
    match workspaces.as_slice() {
        [] => Err(String::from("no workspaces found")),
        [single] => Ok(single.clone()),
        _ => Err(format!(
            "multiple workspaces found; specify one: {}",
            workspaces.join(", ")
        )),
    }
}

fn ensure_image(image: &str, pull: bool) -> Result<(), String> {
    let inspect = Command::new("docker")
        .args(["image", "inspect", image])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if inspect.map(|s| s.success()).unwrap_or(false) {
        return Ok(());
    }

    if !pull {
        return Err(format!(
            "image not found locally: {image} (re-run without --no-pull)"
        ));
    }

    docker_status(&["pull", image])
}

fn create_workspace_container(
    container: &str,
    image: &str,
    primary_repo: Option<&RepoSpec>,
) -> Result<(), String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
        .to_string();

    let (vol_work, vol_home, vol_codex) = volume_names(container);
    let mut cmd = Command::new("docker");
    cmd.arg("run")
        .arg("-d")
        .arg("--name")
        .arg(container)
        .arg("--hostname")
        .arg(container)
        .arg("--label")
        .arg(WORKSPACE_LABEL)
        .arg("--label")
        .arg(format!("agent-kit.created-at={timestamp}"))
        .arg("-e")
        .arg("HOME=/home/agent")
        .arg("-e")
        .arg("AGENT_HOME=/home/agent/.agents")
        .arg("-e")
        .arg("CODEX_AUTH_FILE=/home/agent/.agents/auth.json")
        .arg("-e")
        .arg("ZSH_KIT_DIR=~/.config/zsh")
        .arg("-e")
        .arg("AGENT_KIT_DIR=~/.agents")
        .arg("-e")
        .arg("ZDOTDIR=/home/agent/.config/zsh")
        .arg("-v")
        .arg(format!("{vol_work}:/work"))
        .arg("-v")
        .arg(format!("{vol_home}:/home/agent"))
        .arg("-v")
        .arg(format!("{vol_codex}:/home/agent/.agents"))
        .arg("-w")
        .arg("/work")
        .arg("--entrypoint")
        .arg("bash");

    if let Some(repo) = primary_repo {
        cmd.arg("--label")
            .arg(format!("agent-kit.repo={}", repo.owner_repo));
    }

    cmd.arg(image).arg("-lc").arg("sleep infinity");

    let output = cmd
        .output()
        .map_err(|err| format!("failed to run docker run: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(String::from(
                "docker run failed to create workspace container",
            ));
        }
        return Err(stderr);
    }

    let _ = Command::new("docker")
        .args([
            "exec",
            "-u",
            "root",
            container,
            "bash",
            "-lc",
            "mkdir -p /work && (chown -R agent:agent /work || chown -R codex:codex /work || true)",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    Ok(())
}

fn clone_repo_into_container(
    container: &str,
    repo: &RepoSpec,
    destination: &str,
    refspec: Option<&str>,
) -> Result<(), String> {
    let mut cmd = Command::new("docker");
    cmd.arg("exec");

    if let Ok(token) = std::env::var("GH_TOKEN")
        && !token.trim().is_empty()
    {
        cmd.arg("-e").arg(format!("GH_TOKEN={token}"));
    }
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.trim().is_empty()
    {
        cmd.arg("-e").arg(format!("GITHUB_TOKEN={token}"));
    }

    cmd.arg(container)
        .arg("bash")
        .arg("-lc")
        .arg(CLONE_SCRIPT)
        .arg("--")
        .arg(&repo.clone_url)
        .arg(destination)
        .arg(refspec.unwrap_or(""));

    let output = cmd
        .output()
        .map_err(|err| format!("failed to run git clone for {}: {err}", repo.owner_repo))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!(
                "git clone failed for {} (exit {})",
                repo.owner_repo,
                output.status.code().unwrap_or(EXIT_RUNTIME)
            ))
        } else {
            Err(stderr)
        }
    }
}

fn sync_container_baseline(container: &str) -> Result<(), String> {
    let mut cmd = Command::new("docker");
    cmd.arg("exec");

    for env_name in [
        "AGENT_WORKSPACE_ZSH_KIT_REPO",
        "AGENT_WORKSPACE_AGENT_KIT_REPO",
        "AGENT_WORKSPACE_NILS_CLI_FORMULA",
    ] {
        if let Ok(value) = std::env::var(env_name)
            && !value.trim().is_empty()
        {
            cmd.arg("-e").arg(format!("{env_name}={value}"));
        }
    }

    cmd.arg(container)
        .arg("bash")
        .arg("-lc")
        .arg(SYNC_BASELINE_SCRIPT);

    let output = cmd
        .output()
        .map_err(|err| format!("failed to sync baseline in container: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            return Err(stderr);
        }
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            return Err(stdout);
        }
        Err(format!(
            "baseline sync failed (exit {})",
            output.status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

fn run_auth_github(container: &str, host: Option<&str>) -> i32 {
    let gh_host = host
        .and_then(trimmed_nonempty)
        .or_else(|| std::env::var("GITHUB_HOST").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| String::from("github.com"));

    let auth_mode = std::env::var("AGENT_WORKSPACE_AUTH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("CODEX_WORKSPACE_AUTH").ok())
        .unwrap_or_else(|| String::from("auto"));

    let env_token = std::env::var("GH_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("GITHUB_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    let keyring_token = gh_keyring_token(&gh_host);

    let (chosen_token, chosen_source) = match auth_mode.as_str() {
        "none" => (None, "none"),
        "env" => (env_token, "env"),
        "gh" | "keyring" => {
            if let Some(token) = keyring_token {
                (Some(token), "gh")
            } else {
                eprintln!(
                    "warn: AGENT_WORKSPACE_AUTH={auth_mode} but no gh keyring token found; falling back to GH_TOKEN/GITHUB_TOKEN"
                );
                (env_token, "env")
            }
        }
        "auto" | "" => {
            if let Some(token) = keyring_token {
                (Some(token), "gh")
            } else {
                (env_token, "env")
            }
        }
        _ => {
            eprintln!(
                "error: unknown AGENT_WORKSPACE_AUTH={auth_mode} (expected: auto|gh|env|none)"
            );
            return EXIT_RUNTIME;
        }
    };

    let token = if let Some(token) = chosen_token {
        token
    } else {
        if auth_mode == "none" {
            eprintln!("error: AGENT_WORKSPACE_AUTH=none; no token to apply");
        } else {
            eprintln!("error: no GitHub token found (gh keyring or GH_TOKEN/GITHUB_TOKEN)");
        }
        eprintln!("hint: run 'gh auth login' or export GH_TOKEN/GITHUB_TOKEN");
        return EXIT_RUNTIME;
    };

    let content = format!("host={gh_host}\ntoken={token}\n");
    let target = "/home/agent/.agents/auth/github.env";
    if let Err(err) = write_container_file(container, target, content.as_bytes()) {
        eprintln!("error: failed to write GitHub auth file in container: {err}");
        return EXIT_RUNTIME;
    }

    println!(
        "auth: github -> {} ({gh_host}; source={chosen_source})",
        container
    );
    0
}

fn run_auth_codex(container: &str, profile_arg: Option<&str>) -> i32 {
    let profile = profile_arg
        .and_then(trimmed_nonempty)
        .or_else(|| {
            std::env::var("AGENT_WORKSPACE_CODEX_PROFILE")
                .ok()
                .and_then(|value| trimmed_nonempty(&value))
        })
        .or_else(|| {
            std::env::var("CODEX_WORKSPACE_CODEX_PROFILE")
                .ok()
                .and_then(|value| trimmed_nonempty(&value))
        });

    if let Some(profile) = profile.as_deref()
        && (profile.contains('/')
            || profile.contains("..")
            || profile.chars().any(char::is_whitespace))
    {
        eprintln!("error: invalid codex profile name: {profile}");
        return EXIT_RUNTIME;
    }

    let mut candidate_files: Vec<String> = Vec::new();
    if let Some(profile) = profile.as_deref() {
        candidate_files.extend(resolve_codex_profile_auth_files(profile));
    }
    candidate_files.push(resolve_codex_auth_file());

    for candidate in candidate_files {
        if !std::path::Path::new(&candidate).is_file() {
            continue;
        }

        let auth_data = match std::fs::read(&candidate) {
            Ok(data) => data,
            Err(err) => {
                eprintln!(
                    "warn: failed to read codex auth candidate {}: {err}",
                    candidate
                );
                continue;
            }
        };

        let mut wrote_any_target = false;
        for target in [
            "/home/agent/.codex/auth.json",
            "/home/agent/.agents/auth.json",
        ] {
            if let Err(err) = write_container_file(container, target, &auth_data) {
                eprintln!(
                    "warn: failed to sync codex auth into container target {}: {err}",
                    target
                );
                continue;
            }
            wrote_any_target = true;
        }

        if !wrote_any_target {
            eprintln!(
                "warn: failed to sync codex auth into any known target for source={}",
                candidate
            );
            continue;
        }

        if let Some(profile) = profile.as_deref() {
            println!(
                "auth: codex -> {} (profile={profile}; source={})",
                container, candidate
            );
        } else {
            println!("auth: codex -> {} (source={})", container, candidate);
        }
        return 0;
    }

    eprintln!("error: unable to resolve codex auth file");
    eprintln!("hint: set CODEX_AUTH_FILE or pass --profile <name>");
    EXIT_RUNTIME
}

fn run_auth_gpg(container: &str, key_arg: Option<&str>) -> i32 {
    let key = key_arg
        .and_then(trimmed_nonempty)
        .or_else(default_gpg_signing_key);

    let key = if let Some(key) = key {
        key
    } else {
        eprintln!("error: missing gpg signing key");
        eprintln!("hint: pass --key <fingerprint> or set AGENT_WORKSPACE_GPG_KEY");
        eprintln!("hint: or set: git config --global user.signingkey <keyid>");
        return EXIT_RUNTIME;
    };

    let mut verify = Command::new("docker");
    verify
        .arg("exec")
        .arg(container)
        .arg("bash")
        .arg("-lc")
        .arg("if command -v gpg >/dev/null 2>&1; then gpg --batch --list-secret-keys \"$1\" >/dev/null 2>&1; else exit 0; fi")
        .arg("--")
        .arg(&key);

    match verify.status() {
        Ok(status) if status.success() => {}
        Ok(_) => {
            eprintln!("error: gpg key not found in container keyring: {key}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: failed to run gpg key lookup in container: {err}");
            return EXIT_RUNTIME;
        }
    }

    let target = "/home/agent/.agents/auth/gpg-key.txt";
    if let Err(err) = write_container_file(container, target, format!("{key}\n").as_bytes()) {
        eprintln!("error: failed to write gpg auth file in container: {err}");
        return EXIT_RUNTIME;
    }

    println!("auth: gpg -> {} (key={key})", container);
    0
}

fn write_container_file(container: &str, path: &str, contents: &[u8]) -> Result<(), String> {
    let mut child = Command::new("docker")
        .arg("exec")
        .arg("-i")
        .arg(container)
        .arg("bash")
        .arg("-lc")
        .arg("set -euo pipefail; target=\"$1\"; mkdir -p \"$(dirname \"$target\")\"; cat >\"$target\"; chmod 600 \"$target\" 2>/dev/null || true")
        .arg("--")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to run docker exec for file write: {err}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(contents)
            .map_err(|err| format!("failed to stream file contents to container: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("failed to wait for docker exec: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(String::from("container file write failed"))
        } else {
            Err(stderr)
        }
    }
}

fn gh_keyring_token(host: &str) -> Option<String> {
    if !command_exists("gh") {
        return None;
    }

    let output = Command::new("gh")
        .args(["auth", "token", "-h", host])
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    trimmed_nonempty(String::from_utf8_lossy(&output.stdout).as_ref())
}

fn reset_repo_in_container(container: &str, repo_dir: &str, refspec: &str) -> Result<(), String> {
    let mut cmd = Command::new("docker");
    cmd.arg("exec")
        .arg(container)
        .arg("bash")
        .arg("-lc")
        .arg(RESET_REPO_SCRIPT)
        .arg("--")
        .arg(repo_dir)
        .arg(refspec);

    let status = cmd
        .status()
        .map_err(|err| format!("failed to run reset command in container: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "reset failed for {repo_dir} (exit {})",
            status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

fn list_git_repos_in_container(
    container: &str,
    root: &str,
    depth: u32,
) -> Result<Vec<String>, String> {
    let mut cmd = Command::new("docker");
    cmd.arg("exec")
        .arg(container)
        .arg("bash")
        .arg("-lc")
        .arg(LIST_GIT_REPOS_SCRIPT)
        .arg("--")
        .arg(root)
        .arg(depth.to_string());

    let output = cmd
        .output()
        .map_err(|err| format!("failed to list git repos in container: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return if stderr.is_empty() {
            Err(String::from("failed to list git repos in container"))
        } else {
            Err(stderr)
        };
    }

    let mut repos: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    repos.sort();
    Ok(repos)
}

fn detect_private_repo_dir(container: &str) -> Result<Option<String>, String> {
    let output = docker_output(&[
        "exec",
        container,
        "bash",
        "-lc",
        "set -euo pipefail; find -L /work/private -maxdepth 4 -mindepth 2 \\( -type d -o -type f \\) -name .git -print 2>/dev/null | head -n 1",
    ])?;

    let entry = output.trim();
    if entry.is_empty() {
        return Ok(None);
    }

    Ok(Some(entry.trim_end_matches("/.git").to_string()))
}

fn map_container_repo_path(raw: &str, default_root: &str) -> String {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return String::from(default_root);
    }
    if cleaned.starts_with('/') {
        return cleaned.to_string();
    }

    let without_prefix = cleaned.trim_start_matches("./");
    let root = default_root.trim_end_matches('/');
    format!("{root}/{without_prefix}")
}

fn volume_names(container: &str) -> (String, String, String) {
    (
        format!("{container}-work"),
        format!("{container}-home"),
        format!("{container}-agent-home"),
    )
}

fn sanitize_tunnel_name(input: &str) -> String {
    let lowered = input.to_ascii_lowercase();
    let mut out = String::new();

    for ch in lowered.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }

    let out = out.trim_matches('-');
    let out = if out.is_empty() { "ws" } else { out };

    out.chars().take(20).collect()
}

fn default_tunnel_name(container: &str) -> String {
    let prefix = primary_workspace_prefix();
    let mut candidate = container.to_string();
    let prefixed = format!("{prefix}-");
    if candidate.starts_with(&prefixed) {
        candidate = candidate[prefixed.len()..].to_string();
    }

    if let Some(stripped) = candidate.strip_suffix("-00000000-000000") {
        candidate = stripped.to_string();
    }

    sanitize_tunnel_name(&candidate)
}

fn print_create_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} create [--runtime <container|host>] [--name <workspace>] [--image <image>] [--ref <git-ref>] [--private-repo OWNER/REPO] [--no-work-repos] [--no-extras] [--no-pull] [repo] [extra_repos...]"
    );
}

fn print_ls_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} ls [--runtime <container|host>] [--json|--output json]"
    );
}

fn print_exec_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} exec [--runtime <container|host>] [--root|--user <user>] <workspace> [command ...]"
    );
}

fn print_rm_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} rm [--runtime <container|host>] [--all] [--yes] [--keep-volumes] <workspace>"
    );
}

fn print_tunnel_usage() {
    println!("usage:");
    println!(
        "  {PRIMARY_COMMAND_NAME} tunnel [--runtime <container|host>] <workspace> [--name <tunnel_name>] [--detach] [--output json]"
    );
}

fn print_auth_usage() {
    eprintln!("usage:");
    eprintln!("  {PRIMARY_COMMAND_NAME} auth codex [--profile <name>] [--container <workspace>]");
    eprintln!("  {PRIMARY_COMMAND_NAME} auth github [--host <host>] [--container <workspace>]");
    eprintln!(
        "  {PRIMARY_COMMAND_NAME} auth gpg [--key <keyid|fingerprint>] [--container <workspace>]"
    );
}

fn print_reset_usage() {
    eprintln!("usage:");
    eprintln!(
        "  {PRIMARY_COMMAND_NAME} reset repo <workspace> <repo_dir> [--ref <remote/branch>] [--yes]"
    );
    eprintln!(
        "  {PRIMARY_COMMAND_NAME} reset work-repos <workspace> [--root <path>] [--depth <n>] [--ref <remote/branch>] [--yes]"
    );
    eprintln!("  {PRIMARY_COMMAND_NAME} reset opt-repos <workspace> [--yes]");
    eprintln!(
        "  {PRIMARY_COMMAND_NAME} reset private-repo <workspace> [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_repo_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} reset repo <workspace> <repo_dir> [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_work_repos_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} reset work-repos <workspace> [--root <path>] [--depth <n>] [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_opt_repos_usage() {
    eprintln!("usage: {PRIMARY_COMMAND_NAME} reset opt-repos <workspace> [--yes]");
}

fn print_reset_private_repo_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} reset private-repo <workspace> [--ref <remote/branch>] [--yes]"
    );
}
