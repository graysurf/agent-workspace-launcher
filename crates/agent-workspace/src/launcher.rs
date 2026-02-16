use std::ffi::OsString;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::EXIT_RUNTIME;

pub const DEFAULT_LAUNCHER_PATH: &str = "/opt/agent-kit/docker/agent-env/bin/agent-workspace";
const LAUNCHER_ENV: &str = "AGENT_WORKSPACE_LAUNCHER";

const DEFAULT_CONTAINER_USER: &str = "agent";
const DEFAULT_REF: &str = "origin/main";

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
  echo "error: root not found: $root" >&2
  exit 1
fi

git_depth=$((depth + 1))
find -L "$root" -maxdepth "$git_depth" -mindepth 2 \( -type d -o -type f \) -name .git -print0 2>/dev/null \
  | while IFS= read -r -d '' git_entry; do
      printf '%s\n' "${git_entry%/.git}"
    done \
  | sort -u
"#;

pub fn dispatch(subcommand: &str, args: &[OsString]) -> i32 {
    match subcommand {
        "auth" => run_auth(args),
        "create" => run_create(args),
        "exec" => run_exec(args),
        "reset" => run_reset(args),
        "rm" => run_rm(args),
        "tunnel" => run_tunnel(args),
        _ => forward(subcommand, args),
    }
}

pub fn forward(subcommand: &str, args: &[OsString]) -> i32 {
    let launcher = resolve_launcher_path();
    forward_with_launcher_and_env(&launcher, subcommand, args, &[])
}

#[derive(Debug, Default, Clone)]
struct ParsedCreate {
    show_help: bool,
    no_extras: bool,
    no_work_repos: bool,
    private_repo: Option<String>,
    workspace_name: Option<String>,
    primary_repo: Option<String>,
    extra_repos: Vec<String>,
    forwarded_args: Vec<OsString>,
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
                    parsed.forwarded_args.push(current);
                    idx += 1;
                    continue;
                }
                "--no-work-repos" => {
                    parsed.no_work_repos = true;
                    parsed.forwarded_args.push(OsString::from("--no-clone"));
                    idx += 1;
                    continue;
                }
                "--no-extras" => {
                    parsed.no_extras = true;
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
                    parsed.forwarded_args.push(OsString::from("--name"));
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --name"));
                    }
                    let value = args[idx].to_string_lossy().into_owned();
                    let normalized_name = normalize_workspace_name_for_create(&value);
                    parsed.workspace_name = trimmed_nonempty(&normalized_name);
                    parsed.forwarded_args.push(OsString::from(normalized_name));
                    idx += 1;
                    continue;
                }
                "--" => {
                    positional_only = true;
                    parsed.forwarded_args.push(current);
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
                    let normalized_name = normalize_workspace_name_for_create(value);
                    parsed.workspace_name = trimmed_nonempty(&normalized_name);
                    parsed
                        .forwarded_args
                        .push(OsString::from(format!("--name={normalized_name}")));
                    idx += 1;
                    continue;
                }
                _ if text.starts_with('-') => {
                    parsed.forwarded_args.push(current);
                    idx += 1;
                    continue;
                }
                _ => {}
            }
        }

        if parsed.primary_repo.is_none() {
            parsed.primary_repo = Some(text);
            parsed.forwarded_args.push(current);
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

#[derive(Debug)]
struct CapturedForward {
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug, Clone)]
struct RepoSpec {
    owner: String,
    repo: String,
    owner_repo: String,
    clone_url: String,
}

fn run_create(args: &[OsString]) -> i32 {
    let parsed = match parse_create_args(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let launcher = resolve_launcher_path();
    let before = workspace_container_names();
    let captured = match forward_with_launcher_and_env_capture(
        &launcher,
        "create",
        &parsed.forwarded_args,
        &[],
    ) {
        Ok(captured) => captured,
        Err(err) => {
            eprintln!("{err}");
            return EXIT_RUNTIME;
        }
    };

    if !captured.stdout.is_empty() {
        let _ = std::io::stdout().write_all(&captured.stdout);
        let _ = std::io::stdout().flush();
    }
    if !captured.stderr.is_empty() {
        let _ = std::io::stderr().write_all(&captured.stderr);
        let _ = std::io::stderr().flush();
    }

    if captured.exit_code != 0 || parsed.show_help {
        return captured.exit_code;
    }

    if parsed.no_extras || (parsed.private_repo.is_none() && parsed.extra_repos.is_empty()) {
        return 0;
    }

    let stdout_text = String::from_utf8_lossy(&captured.stdout).to_string();
    let mut workspace =
        parse_workspace_name_from_create_output(&stdout_text).filter(|name| !name.is_empty());
    if workspace.is_none() {
        workspace = parse_workspace_name_from_json(&stdout_text);
    }
    if workspace.is_none() {
        workspace = detect_new_workspace_name(&before);
    }
    if workspace.is_none()
        && let Some(name) = parsed.workspace_name.as_deref()
    {
        let resolved = resolve_workspace_container_name_str(name);
        if docker_container_exists(&resolved) {
            workspace = Some(resolved);
        }
    }

    let Some(container) = workspace else {
        eprintln!("warn: unable to detect workspace name; skipping extra repo setup");
        return 0;
    };

    if let Err(err) = ensure_container_running(&container) {
        eprintln!("warn: {err}");
        eprintln!("warn: skipping extra repo setup");
        return 0;
    }

    let default_host = std::env::var("GITHUB_HOST").unwrap_or_else(|_| String::from("github.com"));
    if let Some(private_repo_raw) = parsed.private_repo.as_deref() {
        if let Some(spec) = parse_repo_spec(private_repo_raw, &default_host) {
            if let Err(err) = setup_private_repo(&container, &spec) {
                eprintln!("warn: {err}");
            }
        } else {
            eprintln!(
                "warn: invalid private repo (expected OWNER/REPO or URL): {private_repo_raw}"
            );
        }
    }

    for extra_repo_raw in &parsed.extra_repos {
        if let Some(spec) = parse_repo_spec(extra_repo_raw, &default_host) {
            if let Err(err) = clone_extra_repo(&container, &spec) {
                eprintln!("warn: {err}");
            }
        } else {
            eprintln!("warn: invalid repo (expected OWNER/REPO or URL): {extra_repo_raw}");
        }
    }

    0
}

fn workspace_container_names() -> Vec<String> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "label=agent-kit.workspace=1",
            "--format",
            "{{.Names}}",
        ])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn detect_new_workspace_name(before: &[String]) -> Option<String> {
    let after = workspace_container_names();
    let mut created: Vec<String> = after
        .into_iter()
        .filter(|candidate| !before.iter().any(|known| known == candidate))
        .collect();
    created.sort();
    if created.len() == 1 {
        created.into_iter().next()
    } else {
        None
    }
}

fn parse_workspace_name_from_create_output(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("workspace:") {
            return trimmed_nonempty(value);
        }
    }
    None
}

fn parse_workspace_name_from_json(stdout: &str) -> Option<String> {
    let key = "\"workspace\"";
    let start = stdout.find(key)?;
    let rest = &stdout[start + key.len()..];
    let colon = rest.find(':')?;
    let mut value = rest[colon + 1..].trim_start();
    if !value.starts_with('"') {
        return None;
    }
    value = &value[1..];
    let end = value.find('"')?;
    trimmed_nonempty(&value[..end])
}

fn parse_repo_spec(input: &str, default_host: &str) -> Option<RepoSpec> {
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return None;
    }

    let mut host = default_host.to_string();
    let mut owner_repo = cleaned.to_string();

    if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
        let without_scheme = cleaned.split_once("://")?.1;
        let (parsed_host, parsed_owner_repo) = without_scheme.split_once('/')?;
        host = parsed_host.to_string();
        owner_repo = parsed_owner_repo.to_string();
    } else if let Some(without_user) = cleaned.strip_prefix("git@") {
        let (parsed_host, parsed_owner_repo) = without_user.split_once(':')?;
        host = parsed_host.to_string();
        owner_repo = parsed_owner_repo.to_string();
    } else if let Some(without_prefix) = cleaned.strip_prefix("ssh://git@") {
        let (parsed_host, parsed_owner_repo) = without_prefix.split_once('/')?;
        host = parsed_host.to_string();
        owner_repo = parsed_owner_repo.to_string();
    }

    owner_repo = owner_repo
        .trim_end_matches(".git")
        .trim_end_matches('/')
        .to_string();
    let mut pieces = owner_repo.split('/');
    let owner = pieces.next()?.trim().to_string();
    let repo = pieces.next()?.trim().to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    let owner_repo = format!("{owner}/{repo}");
    let clone_url = format!("https://{host}/{owner}/{repo}.git");
    Some(RepoSpec {
        owner,
        repo,
        owner_repo,
        clone_url,
    })
}

fn run_container_setup_script(container: &str, script: &str, args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new("docker");
    cmd.args([
        "exec",
        "-u",
        DEFAULT_CONTAINER_USER,
        container,
        "bash",
        "-lc",
        script,
        "--",
    ]);
    cmd.args(args);

    let output = cmd
        .output()
        .map_err(|err| format!("failed to run setup in {container}: {err}"))?;

    if !output.stdout.is_empty() {
        let _ = std::io::stdout().write_all(&output.stdout);
        let _ = std::io::stdout().flush();
    }
    if !output.stderr.is_empty() {
        let _ = std::io::stderr().write_all(&output.stderr);
        let _ = std::io::stderr().flush();
    }

    if !output.status.success() {
        let code = output.status.code().unwrap_or(EXIT_RUNTIME);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "workspace setup failed in {container} (exit {code}): {stderr}"
        ));
    }
    Ok(())
}

fn setup_private_repo(container: &str, repo: &RepoSpec) -> Result<(), String> {
    run_container_setup_script(
        container,
        r#"
set -euo pipefail
repo_url="${1:?missing repo_url}"
owner_repo="${2:?missing owner_repo}"
target="$HOME/.private"

if [[ -d "$target/.git" ]]; then
  printf '%s\n' "+ pull ${owner_repo} -> ~/.private"
  git -C "$target" pull --ff-only || true
  exit 0
fi

if [[ -e "$target" ]]; then
  printf '%s\n' "warn: $target exists but is not a git repo; skipping clone" >&2
  exit 0
fi

printf '%s\n' "+ clone ${owner_repo} -> ~/.private"
GIT_TERMINAL_PROMPT=0 git clone --progress "$repo_url" "$target"
if [[ ! -L /opt/zsh-kit/.private ]]; then
  rm -rf /opt/zsh-kit/.private || true
  ln -s "$HOME/.private" /opt/zsh-kit/.private || true
fi
"#,
        &[repo.clone_url.as_str(), repo.owner_repo.as_str()],
    )
    .map_err(|err| format!("failed to setup ~/.private from {}: {err}", repo.owner_repo))
}

fn clone_extra_repo(container: &str, repo: &RepoSpec) -> Result<(), String> {
    let destination = format!("/work/{}/{}", repo.owner, repo.repo);
    run_container_setup_script(
        container,
        r#"
set -euo pipefail
repo_url="${1:?missing repo_url}"
owner_repo="${2:?missing owner_repo}"
dest="${3:?missing dest}"

if [[ -d "${dest%/}/.git" ]]; then
  printf '%s\n' "repo already present: $dest"
  exit 0
fi

if [[ -e "$dest" ]]; then
  printf '%s\n' "warn: $dest exists but is not a git repo; skipping clone" >&2
  exit 0
fi

printf '%s\n' "+ clone ${owner_repo} -> $dest"
mkdir -p "$(dirname "$dest")"
GIT_TERMINAL_PROMPT=0 git clone --progress "$repo_url" "$dest"
"#,
        &[
            repo.clone_url.as_str(),
            repo.owner_repo.as_str(),
            destination.as_str(),
        ],
    )
    .map_err(|err| format!("failed to clone extra repo {}: {err}", repo.owner_repo))
}

#[derive(Debug, Default, Clone)]
struct ParsedExec {
    show_help: bool,
    user: Option<OsString>,
    workspace: Option<OsString>,
    command: Vec<OsString>,
}

fn parse_exec_args(args: &[OsString]) -> Result<ParsedExec, String> {
    let mut parsed = ParsedExec::default();
    let mut idx = 0usize;

    while idx < args.len() {
        if parsed.workspace.is_some() {
            parsed.command.extend(args[idx..].iter().cloned());
            break;
        }

        let current = &args[idx];
        let text = current.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => {
                parsed.show_help = true;
                return Ok(parsed);
            }
            "--root" => {
                if parsed.user.is_none() {
                    parsed.user = Some(OsString::from("0"));
                }
            }
            "--user" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --user"));
                }
                parsed.user = Some(args[idx].clone());
            }
            _ if text.starts_with("--user=") => {
                parsed.user = Some(OsString::from(&text["--user=".len()..]));
            }
            _ if text.starts_with('-') => {
                return Err(format!("unknown option for exec: {text}"));
            }
            _ => {
                parsed.workspace = Some(current.clone());
            }
        }
        idx += 1;
    }

    if parsed.workspace.is_none() {
        return Err(String::from("missing workspace name"));
    }
    Ok(parsed)
}

fn run_exec(args: &[OsString]) -> i32 {
    let parsed = match parse_exec_args(args) {
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

    let workspace = parsed.workspace.expect("workspace checked");
    let workspace = resolve_workspace_container_name(&workspace);
    let workspace_name = workspace.to_string_lossy().into_owned();
    if let Err(err) = ensure_container_running(&workspace_name) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let user = parsed
        .user
        .unwrap_or_else(|| OsString::from(DEFAULT_CONTAINER_USER));

    let mut cmd = Command::new("docker");
    cmd.arg("exec");
    cmd.arg("-u");
    cmd.arg(user);

    let stdin_tty = std::io::stdin().is_terminal();
    let stdout_tty = std::io::stdout().is_terminal();
    if stdin_tty && stdout_tty {
        cmd.arg("-it");
    } else if stdin_tty {
        cmd.arg("-i");
    }

    if parsed.command.is_empty() {
        cmd.args(["-w", "/work"]);
    }

    cmd.arg(&workspace_name);

    if parsed.command.is_empty() {
        cmd.args(["zsh", "-l"]);
    } else {
        cmd.args(parsed.command);
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(EXIT_RUNTIME),
        Err(err) => {
            eprintln!("error: failed to run docker exec: {err}");
            EXIT_RUNTIME
        }
    }
}

fn run_tunnel(args: &[OsString]) -> i32 {
    if args.iter().any(|arg| {
        let text = arg.to_string_lossy();
        text == "-h" || text == "--help"
    }) {
        print_tunnel_usage();
        return 0;
    }
    forward("tunnel", args)
}

fn print_tunnel_usage() {
    println!("usage:");
    println!(
        "  agent-workspace tunnel <name|container> [--name <tunnel_name>] [--detach] [--output json]"
    );
}

#[derive(Debug, Default, Clone)]
struct ParsedRm {
    show_help: bool,
    all: bool,
    workspace: Option<OsString>,
}

fn parse_rm_args(args: &[OsString]) -> Result<ParsedRm, String> {
    let mut parsed = ParsedRm::default();

    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--all" => parsed.all = true,
            "--yes" => {}
            _ if text.starts_with('-') => {
                return Err(format!("unknown option for rm: {text}"));
            }
            _ => {
                if parsed.workspace.is_some() {
                    return Err(String::from("rm accepts at most one workspace name"));
                }
                parsed.workspace = Some(arg.clone());
            }
        }
    }

    Ok(parsed)
}

fn run_rm(args: &[OsString]) -> i32 {
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

    if parsed.all {
        let workspaces = match list_workspaces() {
            Ok(items) => items,
            Err(err) => {
                eprintln!("error: {err}");
                return EXIT_RUNTIME;
            }
        };
        for workspace in workspaces {
            let code = forward("rm", &[OsString::from(workspace)]);
            if code != 0 {
                return code;
            }
        }
        return 0;
    }

    if let Some(workspace) = parsed.workspace {
        return forward("rm", &[workspace]);
    }

    eprintln!("error: missing workspace name or --all");
    print_rm_usage();
    EXIT_RUNTIME
}

#[derive(Debug, Default, Clone)]
struct ParsedAuth {
    show_help: bool,
    provider: Option<String>,
    container: Option<String>,
    profile: Option<String>,
    host: Option<String>,
    key: Option<String>,
}

fn parse_auth_args(args: &[OsString]) -> Result<ParsedAuth, String> {
    let mut parsed = ParsedAuth::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--container" | "--name" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(format!("missing value for {}", current));
                }
                parsed.container = Some(args[idx].to_string_lossy().into_owned());
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
                parsed.container = Some(current["--container=".len()..].to_string());
            }
            _ if current.starts_with("--name=") => {
                parsed.container = Some(current["--name=".len()..].to_string());
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
            "--" => {
                idx += 1;
                while idx < args.len() {
                    let text = args[idx].to_string_lossy().into_owned();
                    if parsed.provider.is_none() {
                        parsed.provider = Some(text);
                    } else if parsed.container.is_none() {
                        parsed.container = Some(text);
                    } else {
                        return Err(format!("unexpected arg: {}", args[idx].to_string_lossy()));
                    }
                    idx += 1;
                }
                break;
            }
            _ if current.starts_with('-') => {
                return Err(format!("unknown option for auth: {current}"));
            }
            _ => {
                let text = current.to_string();
                if parsed.provider.is_none() {
                    parsed.provider = Some(text);
                } else if parsed.container.is_none() {
                    parsed.container = Some(text);
                } else {
                    return Err(format!("unexpected arg: {current}"));
                }
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

fn run_auth(args: &[OsString]) -> i32 {
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

    let container = match resolve_container_for_auth(parsed.container.as_deref()) {
        Ok(container) => container,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let provider = parsed
        .provider
        .expect("provider checked")
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

fn run_auth_github(container: &str, host: Option<&str>) -> i32 {
    let gh_host = host
        .and_then(trimmed_nonempty)
        .or_else(|| std::env::var("GITHUB_HOST").ok())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| String::from("github.com"));

    let auth_mode = std::env::var("AGENT_WORKSPACE_AUTH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| std::env::var("CODEX_WORKSPACE_AUTH").ok())
        .unwrap_or_else(|| String::from("auto"));

    let env_token = std::env::var("GH_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("GITHUB_TOKEN")
                .ok()
                .filter(|v| !v.trim().is_empty())
        });

    let keyring_token = if command_exists("gh") {
        let output = Command::new("gh")
            .args(["auth", "token", "-h", &gh_host])
            .env_remove("GH_TOKEN")
            .env_remove("GITHUB_TOKEN")
            .output();
        match output {
            Ok(result) if result.status.success() => {
                trimmed_nonempty(String::from_utf8_lossy(&result.stdout).as_ref())
            }
            _ => None,
        }
    } else {
        None
    };

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

    if let Err(err) = ensure_container_running(container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    println!("auth: github -> {container} ({gh_host}; source={chosen_source})");

    let script = r#"
set -euo pipefail
host="${1:-github.com}"
IFS= read -r token || exit 2
[[ -n "$token" ]] || exit 2

if command -v gh >/dev/null 2>&1; then
  printf "%s\n" "$token" | gh auth login --hostname "$host" --with-token >/dev/null 2>&1 || true
  gh auth setup-git --hostname "$host" --force >/dev/null 2>&1 || gh auth setup-git --hostname "$host" >/dev/null 2>&1 || true
  gh config set git_protocol https -h "$host" 2>/dev/null || gh config set git_protocol https 2>/dev/null || true
  exit 0
fi

if command -v git >/dev/null 2>&1; then
  token_file="$HOME/.agents-env/gh.token"
  mkdir -p "${token_file%/*}"
  printf "%s\n" "$token" >| "$token_file"
  chmod 600 "$token_file" 2>/dev/null || true
  git config --global "credential.https://${host}.helper" \
    "!f() { echo username=x-access-token; echo password=\$(cat \"$token_file\"); }; f"
fi
"#;

    let mut cmd = Command::new("docker");
    cmd.args([
        "exec",
        "-i",
        "-u",
        DEFAULT_CONTAINER_USER,
        container,
        "bash",
        "-c",
        script,
        "--",
        &gh_host,
    ]);

    match run_command_with_stdin(cmd, format!("{token}\n").as_bytes(), "update GitHub auth") {
        Ok(0) => 0,
        Ok(code) => {
            eprintln!("error: failed to update GitHub auth in {container} (exit {code})");
            EXIT_RUNTIME
        }
        Err(err) => {
            eprintln!("error: {err}");
            EXIT_RUNTIME
        }
    }
}

fn run_auth_codex(container: &str, profile_arg: Option<&str>) -> i32 {
    let profile = profile_arg
        .and_then(trimmed_nonempty)
        .or_else(|| {
            std::env::var("AGENT_WORKSPACE_CODEX_PROFILE")
                .ok()
                .and_then(|v| trimmed_nonempty(&v))
        })
        .or_else(|| {
            std::env::var("CODEX_WORKSPACE_CODEX_PROFILE")
                .ok()
                .and_then(|v| trimmed_nonempty(&v))
        });

    if let Some(profile) = profile {
        if profile.contains('/')
            || profile.contains("..")
            || profile.chars().any(char::is_whitespace)
        {
            eprintln!("error: invalid codex profile name: {profile}");
            return EXIT_RUNTIME;
        }

        if let Err(err) = ensure_container_running(container) {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }

        let script = r#"
profile="${1:?missing profile}"
if ! typeset -f codex-use >/dev/null 2>&1; then
  for source_file in \
    /opt/zsh-kit/scripts/_features/agent-workspace/workspace-launcher.zsh \
    /opt/zsh-kit/scripts/_features/codex-workspace/workspace-launcher.zsh \
    /opt/zsh-kit/scripts/_features/agent-workspace/init.zsh \
    /opt/zsh-kit/scripts/_features/codex-workspace/init.zsh
  do
    if [[ -f "$source_file" ]]; then
      source "$source_file"
    fi
  done

  if (( $+functions[_agent_workspace_require_codex_use] )); then
    _agent_workspace_require_codex_use >/dev/null 2>&1 || true
  fi
  if (( $+functions[_codex_workspace_require_codex_use] )); then
    _codex_workspace_require_codex_use >/dev/null 2>&1 || true
  fi

  if ! typeset -f codex-use >/dev/null 2>&1; then
    for source_file in \
      "$HOME/.config/codex_secrets/codex-secret.zsh" \
      /opt/zsh-kit/scripts/_features/agent-workspace/codex-secret.zsh \
      /opt/zsh-kit/scripts/_features/codex-workspace/codex-secret.zsh \
      /opt/zsh-kit/scripts/_features/codex/codex-secret.zsh
    do
      if [[ -f "$source_file" ]]; then
        source "$source_file"
        break
      fi
    done
  fi
fi
codex-use "$profile"
"#;
        let output = Command::new("docker")
            .args([
                "exec",
                "-u",
                DEFAULT_CONTAINER_USER,
                container,
                "zsh",
                "-lc",
                script,
                "--",
                &profile,
            ])
            .output();
        match output {
            Ok(result) if result.status.success() => {
                println!("auth: codex -> {container} (profile={profile})");
                0
            }
            Ok(result) => {
                let mut fallback_files = resolve_codex_profile_auth_files(&profile);
                let resolved_auth_file = resolve_codex_auth_file();
                if !fallback_files
                    .iter()
                    .any(|path| path == &resolved_auth_file)
                {
                    fallback_files.push(resolved_auth_file);
                }

                for auth_file in fallback_files {
                    if !Path::new(&auth_file).is_file() {
                        continue;
                    }
                    match fs::read(&auth_file) {
                        Ok(auth_data) => {
                            match sync_codex_auth_into_container(container, &auth_data) {
                                Ok(()) => {
                                    println!(
                                        "auth: codex -> {container} (profile={profile}; synced fallback auth: {auth_file})"
                                    );
                                    return 0;
                                }
                                Err(err) => eprintln!("warn: {err}"),
                            }
                        }
                        Err(err) => eprintln!(
                            "warn: failed to read codex auth file for fallback {auth_file}: {err}"
                        ),
                    }
                }
                eprintln!(
                    "error: failed to apply codex profile in {container} (exit {})",
                    result.status.code().unwrap_or(EXIT_RUNTIME)
                );
                eprintln!("hint: ensure codex secrets are mounted and profile exists");
                EXIT_RUNTIME
            }
            Err(err) => {
                eprintln!("error: failed to run docker exec for codex auth: {err}");
                EXIT_RUNTIME
            }
        }
    } else {
        let auth_file = resolve_codex_auth_file();
        if !Path::new(&auth_file).is_file() {
            eprintln!("error: codex auth file not found: {auth_file}");
            eprintln!("hint: set CODEX_AUTH_FILE or pass --profile <name>");
            return EXIT_RUNTIME;
        }

        let auth_data = match fs::read(&auth_file) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("error: failed to read codex auth file {auth_file}: {err}");
                return EXIT_RUNTIME;
            }
        };

        if let Err(err) = ensure_container_running(container) {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }

        match sync_codex_auth_into_container(container, &auth_data) {
            Ok(()) => {
                println!("auth: codex -> {container} (synced auth file)");
                0
            }
            Err(err) => {
                eprintln!("error: {err}");
                EXIT_RUNTIME
            }
        }
    }
}

fn sync_codex_auth_into_container(container: &str, auth_data: &[u8]) -> Result<(), String> {
    let script = r#"
set -euo pipefail
target="${CODEX_AUTH_FILE:-$HOME/.codex/auth.json}"
[[ -n "$target" ]] || target="$HOME/.codex/auth.json"
mkdir -p "$(dirname "$target")"
rm -f -- "$target"
umask 077
cat > "$target"
"#;

    let mut cmd = Command::new("docker");
    cmd.args([
        "exec",
        "-i",
        "-u",
        DEFAULT_CONTAINER_USER,
        container,
        "bash",
        "-c",
        script,
    ]);

    match run_command_with_stdin(cmd, auth_data, "sync codex auth file") {
        Ok(0) => Ok(()),
        Ok(code) => Err(format!(
            "failed to sync codex auth into {container} (exit {code})"
        )),
        Err(err) => Err(err),
    }
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

    if !command_exists("gpg") {
        eprintln!("error: gpg not found on host (required to export secret key)");
        return EXIT_RUNTIME;
    }

    if let Err(err) = ensure_container_running(container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    println!("auth: gpg -> {container} (key={key})");

    let mut export_cmd = Command::new("gpg");
    export_cmd.args(["--batch", "--armor", "--export-secret-keys", &key]);
    export_cmd.stdout(Stdio::piped());

    let mut export_child = match export_cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            eprintln!("error: failed to export gpg key {key}: {err}");
            return EXIT_RUNTIME;
        }
    };

    let export_stdout = match export_child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            eprintln!("error: failed to capture gpg export stdout");
            let _ = export_child.kill();
            return EXIT_RUNTIME;
        }
    };

    let script = r#"
set -euo pipefail
if ! command -v gpg >/dev/null 2>&1; then
  echo "error: gpg not installed in container" >&2
  exit 127
fi
umask 077
mkdir -p "$HOME/.gnupg"
chmod 700 "$HOME/.gnupg" 2>/dev/null || true
gpg --batch --import >/dev/null 2>&1
"#;

    let import_status = Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            DEFAULT_CONTAINER_USER,
            container,
            "bash",
            "-c",
            script,
        ])
        .stdin(Stdio::from(export_stdout))
        .status();

    let export_status = export_child.wait();

    match (export_status, import_status) {
        (Ok(export), Ok(import)) if export.success() && import.success() => {
            let verify_ok = Command::new("docker")
                .args([
                    "exec",
                    "-u",
                    DEFAULT_CONTAINER_USER,
                    container,
                    "gpg",
                    "--list-secret-keys",
                    "--keyid-format",
                    "LONG",
                    "--",
                    &key,
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            if !verify_ok {
                eprintln!(
                    "warn: gpg import completed but key lookup failed in container (key={key})"
                );
            }
            0
        }
        (Ok(export), Ok(import)) => {
            eprintln!(
                "error: failed to import gpg key into {container} (export exit {}, import exit {})",
                export.code().unwrap_or(EXIT_RUNTIME),
                import.code().unwrap_or(EXIT_RUNTIME)
            );
            EXIT_RUNTIME
        }
        (Err(err), _) => {
            eprintln!("error: failed while waiting for gpg export process: {err}");
            EXIT_RUNTIME
        }
        (_, Err(err)) => {
            eprintln!("error: failed to run docker import for gpg auth: {err}");
            EXIT_RUNTIME
        }
    }
}

fn run_reset(args: &[OsString]) -> i32 {
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
            eprintln!("hint: agent-workspace reset --help");
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

    let container_name = if let Some(container) = parsed.container {
        container
    } else {
        eprintln!("error: missing container");
        print_reset_repo_usage();
        return EXIT_RUNTIME;
    };
    let repo_dir = if let Some(repo_dir) = parsed.repo_dir {
        repo_dir
    } else {
        eprintln!("error: missing repo_dir");
        print_reset_repo_usage();
        return EXIT_RUNTIME;
    };

    let container = resolve_workspace_container_name_str(&container_name);
    if !docker_container_exists(&container) {
        eprintln!("error: workspace container not found: {container}");
        return EXIT_RUNTIME;
    }
    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    if !parsed.yes {
        println!("This will reset a repo inside container: {container}");
        println!("  - {repo_dir}");
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    match reset_repo_in_container(&container, &repo_dir, &parsed.refspec) {
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

    let container_name = if let Some(container) = parsed.container {
        container
    } else {
        eprintln!("error: missing container");
        print_reset_work_repos_usage();
        return EXIT_RUNTIME;
    };

    let container = resolve_workspace_container_name_str(&container_name);
    if !docker_container_exists(&container) {
        eprintln!("error: workspace container not found: {container}");
        return EXIT_RUNTIME;
    }
    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let repos = match list_git_repos_in_container(&container, &parsed.root, parsed.depth) {
        Ok(repos) => repos,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if repos.is_empty() {
        eprintln!(
            "warn: no git repos found under {} (depth={}) in {}",
            parsed.root, parsed.depth, container
        );
        return 0;
    }

    if !parsed.yes {
        println!(
            "This will reset {} repos inside container: {}",
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
        if reset_repo_in_container(&container, &repo, &parsed.refspec).is_err() {
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
    let parsed = match parse_reset_opt_repos_args(args) {
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

    let container_name = if let Some(container) = parsed.container {
        container
    } else {
        eprintln!("error: missing container");
        print_reset_opt_repos_usage();
        return EXIT_RUNTIME;
    };

    let container = resolve_workspace_container_name_str(&container_name);
    if !docker_container_exists(&container) {
        eprintln!("error: workspace container not found: {container}");
        return EXIT_RUNTIME;
    }
    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    if !parsed.yes {
        println!("This will reset /opt repos inside container: {container}");
        println!("  - /opt/codex-kit");
        println!("  - /opt/zsh-kit");
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    println!("+ refresh /opt repos in {container}");
    for repo_dir in ["/opt/codex-kit", "/opt/zsh-kit"] {
        let has_repo = match container_has_git_repo(&container, repo_dir) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("error: {err}");
                return EXIT_RUNTIME;
            }
        };
        if has_repo && let Err(err) = reset_repo_in_container(&container, repo_dir, DEFAULT_REF) {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    }

    let _ = Command::new("docker")
        .args([
            "exec",
            "-u",
            DEFAULT_CONTAINER_USER,
            &container,
            "bash",
            "-lc",
            r#"
set -euo pipefail
codex_home="${CODEX_HOME:-/home/agent/.codex}"
if [[ -d /opt/codex-kit ]] && command -v rsync >/dev/null 2>&1; then
  mkdir -p "$codex_home"
  rsync -a --delete --exclude=".git" /opt/codex-kit/ "$codex_home"/ >/dev/null 2>&1 || true
fi
"#,
        ])
        .status();

    0
}

fn run_reset_private_repo(args: &[OsString]) -> i32 {
    let parsed = match parse_reset_private_repo_args(args) {
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

    let container_name = if let Some(container) = parsed.container {
        container
    } else {
        eprintln!("error: missing container");
        print_reset_private_repo_usage();
        return EXIT_RUNTIME;
    };

    let container = resolve_workspace_container_name_str(&container_name);
    if !docker_container_exists(&container) {
        eprintln!("error: workspace container not found: {container}");
        return EXIT_RUNTIME;
    }
    if let Err(err) = ensure_container_running(&container) {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    let private_repo_dir = match detect_private_repo_dir(&container) {
        Ok(Some(dir)) => dir,
        Ok(None) => {
            eprintln!("warn: ~/.private not found (or not a git repo) in container: {container}");
            eprintln!(
                "hint: seed it with: AGENT_WORKSPACE_PRIVATE_REPO=OWNER/REPO agent-workspace create ..."
            );
            return 0;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if !parsed.yes {
        println!("This will reset ~/.private inside container: {container}");
        println!("  - {private_repo_dir}");
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    match reset_repo_in_container(&container, &private_repo_dir, &parsed.refspec) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("error: {err}");
            EXIT_RUNTIME
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedResetRepo {
    show_help: bool,
    container: Option<String>,
    repo_dir: Option<String>,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetRepo {
    fn default() -> Self {
        Self {
            show_help: false,
            container: None,
            repo_dir: None,
            refspec: String::from(DEFAULT_REF),
            yes: false,
        }
    }
}

fn parse_reset_repo_args(args: &[OsString]) -> Result<ParsedResetRepo, String> {
    let mut parsed = ParsedResetRepo::default();
    let mut idx = 0usize;
    while idx < args.len() {
        let text = args[idx].to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with("--ref=") => {
                parsed.refspec = text["--ref=".len()..].to_string();
            }
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.container.is_none() {
                    parsed.container = Some(text.to_string());
                } else if parsed.repo_dir.is_none() {
                    parsed.repo_dir = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }
        idx += 1;
    }
    Ok(parsed)
}

#[derive(Debug, Clone)]
struct ParsedResetWorkRepos {
    show_help: bool,
    container: Option<String>,
    root: String,
    depth: u32,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetWorkRepos {
    fn default() -> Self {
        Self {
            show_help: false,
            container: None,
            root: String::from("/work"),
            depth: 3,
            refspec: String::from(DEFAULT_REF),
            yes: false,
        }
    }
}

fn parse_reset_work_repos_args(args: &[OsString]) -> Result<ParsedResetWorkRepos, String> {
    let mut parsed = ParsedResetWorkRepos::default();
    let mut idx = 0usize;
    while idx < args.len() {
        let text = args[idx].to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--root" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --root"));
                }
                parsed.root = args[idx].to_string_lossy().into_owned();
            }
            "--depth" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --depth"));
                }
                parsed.depth = args[idx]
                    .to_string_lossy()
                    .parse::<u32>()
                    .map_err(|_| String::from("--depth must be a positive integer"))?;
            }
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with("--root=") => parsed.root = text["--root=".len()..].to_string(),
            _ if text.starts_with("--depth=") => {
                parsed.depth = text["--depth=".len()..]
                    .parse::<u32>()
                    .map_err(|_| String::from("--depth must be a positive integer"))?;
            }
            _ if text.starts_with("--ref=") => parsed.refspec = text["--ref=".len()..].to_string(),
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.container.is_none() {
                    parsed.container = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }
        idx += 1;
    }
    if parsed.depth == 0 {
        return Err(String::from("--depth must be a positive integer"));
    }
    Ok(parsed)
}

#[derive(Debug, Default, Clone)]
struct ParsedResetSimple {
    show_help: bool,
    container: Option<String>,
    yes: bool,
}

fn parse_reset_opt_repos_args(args: &[OsString]) -> Result<ParsedResetSimple, String> {
    let mut parsed = ParsedResetSimple::default();
    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.container.is_none() {
                    parsed.container = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }
    }
    Ok(parsed)
}

#[derive(Debug, Clone)]
struct ParsedResetPrivate {
    show_help: bool,
    container: Option<String>,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetPrivate {
    fn default() -> Self {
        Self {
            show_help: false,
            container: None,
            refspec: String::from(DEFAULT_REF),
            yes: false,
        }
    }
}

fn parse_reset_private_repo_args(args: &[OsString]) -> Result<ParsedResetPrivate, String> {
    let mut parsed = ParsedResetPrivate::default();
    let mut idx = 0usize;
    while idx < args.len() {
        let text = args[idx].to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--ref" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --ref"));
                }
                parsed.refspec = args[idx].to_string_lossy().into_owned();
            }
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with("--ref=") => parsed.refspec = text["--ref=".len()..].to_string(),
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.container.is_none() {
                    parsed.container = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }
        idx += 1;
    }
    Ok(parsed)
}

fn print_exec_usage() {
    eprintln!("usage: agent-workspace exec [--root|--user <user>] <workspace> [command ...]");
}

fn print_rm_usage() {
    eprintln!("usage: agent-workspace rm [--all] [--yes] <workspace>");
}

fn print_auth_usage() {
    eprintln!("usage:");
    eprintln!("  agent-workspace auth codex [--profile <name>] [--container <name|container>]");
    eprintln!("  agent-workspace auth github [--host <host>] [--container <name|container>]");
    eprintln!(
        "  agent-workspace auth gpg [--key <keyid|fingerprint>] [--container <name|container>]"
    );
}

fn print_reset_usage() {
    eprintln!("usage:");
    eprintln!(
        "  agent-workspace reset repo <name|container> <repo_dir> [--ref <remote/branch>] [--yes]"
    );
    eprintln!(
        "  agent-workspace reset work-repos <name|container> [--root <dir>] [--depth <N>] [--ref <remote/branch>] [--yes]"
    );
    eprintln!("  agent-workspace reset opt-repos <name|container> [--yes]");
    eprintln!(
        "  agent-workspace reset private-repo <name|container> [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_repo_usage() {
    eprintln!(
        "usage: agent-workspace reset repo <name|container> <repo_dir> [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_work_repos_usage() {
    eprintln!(
        "usage: agent-workspace reset work-repos <name|container> [--root <dir>] [--depth <N>] [--ref <remote/branch>] [--yes]"
    );
}

fn print_reset_opt_repos_usage() {
    eprintln!("usage: agent-workspace reset opt-repos <name|container> [--yes]");
}

fn print_reset_private_repo_usage() {
    eprintln!(
        "usage: agent-workspace reset private-repo <name|container> [--ref <remote/branch>] [--yes]"
    );
}

fn resolve_workspace_container_name(workspace: &OsString) -> OsString {
    OsString::from(resolve_workspace_container_name_str(
        workspace.to_string_lossy().as_ref(),
    ))
}

fn resolve_workspace_container_name_str(workspace_name: &str) -> String {
    if docker_container_exists(workspace_name) {
        return workspace_name.to_string();
    }

    let prefixes = workspace_prefixes();
    for prefix in prefixes {
        let prefixed = if workspace_name.starts_with(&(prefix.clone() + "-")) {
            workspace_name.to_string()
        } else {
            format!("{prefix}-{workspace_name}")
        };
        if docker_container_exists(&prefixed) {
            return prefixed;
        }
    }

    workspace_name.to_string()
}

fn workspace_prefixes() -> Vec<String> {
    let mut prefixes: Vec<String> = Vec::new();
    if let Ok(value) = std::env::var("AGENT_WORKSPACE_PREFIX")
        && let Some(cleaned) = trimmed_nonempty(&value)
    {
        push_unique(&mut prefixes, cleaned);
    }
    if let Ok(value) = std::env::var("CODEX_WORKSPACE_PREFIX")
        && let Some(cleaned) = trimmed_nonempty(&value)
    {
        push_unique(&mut prefixes, cleaned);
    }
    push_unique(&mut prefixes, String::from("agent-ws"));
    push_unique(&mut prefixes, String::from("codex-ws"));
    prefixes
}

fn workspace_name_variants(input: &str, prefixes: &[String]) -> Vec<String> {
    let Some(mut current) = trimmed_nonempty(input) else {
        return Vec::new();
    };

    let mut variants = vec![current.clone()];
    loop {
        let mut stripped: Option<String> = None;

        for prefix in prefixes {
            let prefix = format!("{prefix}-");
            if let Some(rest) = current.strip_prefix(&prefix)
                && let Some(cleaned) = trimmed_nonempty(rest)
            {
                stripped = Some(cleaned);
                break;
            }
        }

        if stripped.is_none()
            && let Some(rest) = current.strip_prefix("ws-")
            && let Some(cleaned) = trimmed_nonempty(rest)
        {
            stripped = Some(cleaned);
        }

        let Some(next) = stripped else {
            break;
        };
        if variants.iter().any(|known| known == &next) {
            break;
        }
        variants.push(next.clone());
        current = next;
    }

    variants
}

fn normalize_workspace_name_for_create(name: &str) -> String {
    let variants = workspace_name_variants(name, &workspace_prefixes());
    variants
        .last()
        .cloned()
        .or_else(|| trimmed_nonempty(name))
        .unwrap_or_default()
}

fn docker_container_exists(name: &str) -> bool {
    Command::new("docker")
        .args(["container", "inspect", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn ensure_container_running(container: &str) -> Result<(), String> {
    if !docker_container_exists(container) {
        return Err(format!("workspace container not found: {container}"));
    }

    let running = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", container])
        .output()
        .map_err(|err| format!("failed to inspect workspace {container}: {err}"))?;
    if !running.status.success() {
        let stderr = String::from_utf8_lossy(&running.stderr).trim().to_string();
        return Err(format!(
            "docker inspect failed for {container} (exit {}): {stderr}",
            running.status.code().unwrap_or(EXIT_RUNTIME)
        ));
    }

    let is_running = String::from_utf8_lossy(&running.stdout).trim().eq("true");
    if is_running {
        return Ok(());
    }

    let status = Command::new("docker")
        .args(["start", container])
        .status()
        .map_err(|err| format!("failed to start workspace {container}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "docker start failed for {container} (exit {})",
            status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

fn resolve_container_for_auth(name: Option<&str>) -> Result<String, String> {
    if let Some(name) = name.and_then(trimmed_nonempty) {
        let container = resolve_workspace_container_name_str(&name);
        if docker_container_exists(&container) {
            return Ok(container);
        }
        return Err(format!("workspace container not found: {container}"));
    }

    let workspaces = list_workspaces()?;
    match workspaces.as_slice() {
        [] => Err(String::from("no workspaces found")),
        [single] => Ok(single.clone()),
        _ => Err(format!(
            "multiple workspaces found; specify one: {}",
            workspaces.join(", ")
        )),
    }
}

fn list_workspaces() -> Result<Vec<String>, String> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "label=agent-kit.workspace=1",
            "--format",
            "{{.Names}}",
        ])
        .output()
        .map_err(|err| format!("failed to list workspaces via docker: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "docker ps failed (exit {}): {stderr}",
            output.status.code().unwrap_or(EXIT_RUNTIME)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn run_command_with_stdin(mut cmd: Command, input: &[u8], context: &str) -> Result<i32, String> {
    cmd.stdin(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("failed to {context}: {err}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input)
            .map_err(|err| format!("failed to write stdin for {context}: {err}"))?;
    }
    let status = child
        .wait()
        .map_err(|err| format!("failed to wait process for {context}: {err}"))?;
    Ok(status.code().unwrap_or(EXIT_RUNTIME))
}

fn reset_repo_in_container(container: &str, repo_dir: &str, refspec: &str) -> Result<(), String> {
    let status = Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            DEFAULT_CONTAINER_USER,
            container,
            "bash",
            "-c",
            RESET_REPO_SCRIPT,
            "--",
            repo_dir,
            refspec,
        ])
        .status()
        .map_err(|err| format!("failed to reset repo {repo_dir} in {container}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to reset repo {repo_dir} in {container} (exit {})",
            status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

fn list_git_repos_in_container(
    container: &str,
    root: &str,
    depth: u32,
) -> Result<Vec<String>, String> {
    let output = Command::new("docker")
        .args([
            "exec",
            "-u",
            DEFAULT_CONTAINER_USER,
            container,
            "bash",
            "-c",
            LIST_GIT_REPOS_SCRIPT,
            "--",
            root,
            &depth.to_string(),
        ])
        .output()
        .map_err(|err| format!("failed to list git repos in {container}: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "failed to list git repos in {container} (exit {}): {stderr}",
            output.status.code().unwrap_or(EXIT_RUNTIME)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn container_has_git_repo(container: &str, repo_dir: &str) -> Result<bool, String> {
    let status = Command::new("docker")
        .args([
            "exec",
            "-u",
            DEFAULT_CONTAINER_USER,
            container,
            "bash",
            "-lc",
            "test -d \"$1/.git\"",
            "--",
            repo_dir,
        ])
        .status()
        .map_err(|err| format!("failed to inspect repo path {repo_dir} in {container}: {err}"))?;
    Ok(status.success())
}

fn detect_private_repo_dir(container: &str) -> Result<Option<String>, String> {
    let output = Command::new("docker")
        .args([
            "exec",
            "-u",
            DEFAULT_CONTAINER_USER,
            container,
            "bash",
            "-lc",
            r#"
set -euo pipefail
for dir in "$HOME/.private" /home/codex/.private /home/agent/.private; do
  if [[ -d "$dir/.git" ]]; then
    printf '%s\n' "$dir"
    exit 0
  fi
done
"#,
        ])
        .output()
        .map_err(|err| format!("failed to inspect private repo path in {container}: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "failed to detect private repo path in {container} (exit {}): {stderr}",
            output.status.code().unwrap_or(EXIT_RUNTIME)
        ));
    }
    let found = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if found.is_empty() {
        Ok(None)
    } else {
        Ok(Some(found))
    }
}

fn command_exists(command: &str) -> bool {
    Command::new("bash")
        .args(["-lc", &format!("command -v {command} >/dev/null 2>&1")])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn resolve_codex_auth_file() -> String {
    if let Ok(value) = std::env::var("CODEX_AUTH_FILE")
        && !value.trim().is_empty()
    {
        return value;
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return format!("{home}/.codex/auth.json");
    }
    String::from("/root/.codex/auth.json")
}

fn resolve_codex_profile_auth_files(profile: &str) -> Vec<String> {
    let profile = match trimmed_nonempty(profile) {
        Some(value) => value,
        None => return Vec::new(),
    };

    let mut dirs: Vec<String> = Vec::new();
    if let Ok(value) = std::env::var("CODEX_SECRET_DIR")
        && !value.trim().is_empty()
    {
        dirs.push(value);
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        dirs.push(format!("{home}/.config/codex_secrets"));
        dirs.push(format!("{home}/codex_secrets"));
    }
    dirs.push(String::from("/home/agent/codex_secrets"));
    dirs.push(String::from("/home/codex/codex_secrets"));
    dirs.push(String::from("/opt/zsh-kit/scripts/_features/codex/secrets"));

    let mut out: Vec<String> = Vec::new();
    for dir in dirs {
        let base = dir.trim_end_matches('/');
        if base.is_empty() {
            continue;
        }
        let candidates = [
            format!("{base}/{profile}.json"),
            format!("{base}/{profile}"),
        ];
        for candidate in candidates {
            if !out.iter().any(|known| known == &candidate) {
                out.push(candidate);
            }
        }
    }
    out
}

fn default_gpg_signing_key() -> Option<String> {
    if let Ok(value) = std::env::var("AGENT_WORKSPACE_GPG_KEY")
        && let Some(cleaned) = trimmed_nonempty(&value)
    {
        return Some(cleaned);
    }

    if let Ok(value) = std::env::var("CODEX_WORKSPACE_GPG_KEY")
        && let Some(cleaned) = trimmed_nonempty(&value)
    {
        return Some(cleaned);
    }

    let output = Command::new("git")
        .args(["config", "--global", "--get", "user.signingkey"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    trimmed_nonempty(String::from_utf8_lossy(&output.stdout).as_ref())
}

fn confirm_or_abort(prompt: &str) -> bool {
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim(), "y" | "Y")
}

fn trimmed_nonempty(input: &str) -> Option<String> {
    let value = input.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|known| known == &value) {
        values.push(value);
    }
}

pub(crate) fn forward_with_launcher_and_env(
    launcher: &Path,
    subcommand: &str,
    args: &[OsString],
    env_overrides: &[(&str, &str)],
) -> i32 {
    if !launcher.is_file() {
        eprintln!("error: launcher not found: {}", launcher.display());
        eprintln!("hint: set {LAUNCHER_ENV} to the low-level launcher path");
        return EXIT_RUNTIME;
    }

    let mut cmd = Command::new(launcher);
    cmd.arg(subcommand);
    cmd.args(args.iter().cloned());
    for (k, v) in env_overrides {
        cmd.env(k, v);
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(EXIT_RUNTIME),
        Err(err) => {
            eprintln!(
                "error: failed to run launcher {}: {err}",
                launcher.display()
            );
            EXIT_RUNTIME
        }
    }
}

fn forward_with_launcher_and_env_capture(
    launcher: &Path,
    subcommand: &str,
    args: &[OsString],
    env_overrides: &[(&str, &str)],
) -> Result<CapturedForward, String> {
    if !launcher.is_file() {
        return Err(format!(
            "error: launcher not found: {}\nhint: set {LAUNCHER_ENV} to the low-level launcher path",
            launcher.display()
        ));
    }

    let mut cmd = Command::new(launcher);
    cmd.arg(subcommand);
    cmd.args(args.iter().cloned());
    for (k, v) in env_overrides {
        cmd.env(k, v);
    }

    let output = cmd.output().map_err(|err| {
        format!(
            "error: failed to run launcher {}: {err}",
            launcher.display()
        )
    })?;

    Ok(CapturedForward {
        exit_code: output.status.code().unwrap_or(EXIT_RUNTIME),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

fn resolve_launcher_path() -> PathBuf {
    launcher_path_from_env(std::env::var_os(LAUNCHER_ENV))
}

fn launcher_path_from_env(value: Option<OsString>) -> PathBuf {
    match value {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => auto_detect_launcher_path(),
    }
}

fn auto_detect_launcher_path() -> PathBuf {
    if Path::new(DEFAULT_LAUNCHER_PATH).is_file() {
        return PathBuf::from(DEFAULT_LAUNCHER_PATH);
    }
    PathBuf::from(DEFAULT_LAUNCHER_PATH)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use super::{
        DEFAULT_LAUNCHER_PATH, forward_with_launcher_and_env, launcher_path_from_env,
        normalize_workspace_name_for_create, parse_auth_args, parse_create_args, parse_exec_args,
        parse_reset_repo_args, parse_rm_args, workspace_name_variants,
    };
    use crate::EXIT_RUNTIME;

    #[test]
    fn launcher_path_defaults_when_env_absent() {
        let path = launcher_path_from_env(None);
        assert_eq!(path, PathBuf::from(DEFAULT_LAUNCHER_PATH));
    }

    #[test]
    fn launcher_path_uses_env_when_present() {
        let path = launcher_path_from_env(Some(OsString::from("/tmp/custom-launcher")));
        assert_eq!(path, PathBuf::from("/tmp/custom-launcher"));
    }

    #[test]
    fn launcher_path_treats_empty_env_as_default() {
        let path = launcher_path_from_env(Some(OsString::from("")));
        assert_eq!(path, PathBuf::from(DEFAULT_LAUNCHER_PATH));
    }

    #[test]
    fn forwarding_passes_subcommand_args_and_codex_env() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let launcher = write_stub_launcher(temp.path());
        let log_path = temp.path().join("launcher.log");
        let log_path_str = log_path.to_string_lossy().to_string();

        let args = vec![
            OsString::from("github"),
            OsString::from("ws-test"),
            OsString::from("--host"),
            OsString::from("github.com"),
        ];

        let exit_code = forward_with_launcher_and_env(
            &launcher,
            "auth",
            &args,
            &[
                ("AW_TEST_LOG", &log_path_str),
                ("CODEX_SECRET_DIR", "/tmp/codex-secrets"),
                ("CODEX_AUTH_FILE", "/tmp/codex-auth.json"),
            ],
        );
        assert_eq!(exit_code, 0);

        let log = fs::read_to_string(log_path).expect("read log");
        for expected in [
            "subcommand=auth",
            "arg0=github",
            "arg1=ws-test",
            "arg2=--host",
            "arg3=github.com",
            "codex_secret_dir=/tmp/codex-secrets",
            "codex_auth_file=/tmp/codex-auth.json",
        ] {
            assert!(log.contains(expected), "missing line: {expected}\n{log}");
        }
    }

    #[test]
    fn forwarding_returns_child_exit_code() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let launcher = write_stub_launcher(temp.path());

        let exit_code =
            forward_with_launcher_and_env(&launcher, "ls", &[], &[("AW_TEST_EXIT_CODE", "17")]);
        assert_eq!(exit_code, 17);
    }

    #[test]
    fn forwarding_fails_when_launcher_is_missing() {
        let path = PathBuf::from("/tmp/agent-workspace-tests/missing-launcher");
        let exit_code = forward_with_launcher_and_env(&path, "ls", &[], &[]);
        assert_eq!(exit_code, EXIT_RUNTIME);
    }

    #[test]
    fn create_translation_maps_no_work_repos_to_no_clone() {
        let translated = parse_create_args(&[
            OsString::from("--no-work-repos"),
            OsString::from("--name"),
            OsString::from("demo"),
        ])
        .expect("parse")
        .forwarded_args;
        let values: Vec<String> = translated
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(values, vec!["--no-clone", "--name", "demo"]);
    }

    #[test]
    fn create_translation_normalizes_ws_prefixed_name() {
        let translated = parse_create_args(&[OsString::from("--name"), OsString::from("ws-test")])
            .expect("parse")
            .forwarded_args;
        let values: Vec<String> = translated
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(values, vec!["--name", "test"]);
    }

    #[test]
    fn create_translation_normalizes_name_equals_syntax() {
        let translated = parse_create_args(&[OsString::from("--name=agent-ws-ws-test")])
            .expect("parse")
            .forwarded_args;
        let values: Vec<String> = translated
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(values, vec!["--name=test"]);
    }

    #[test]
    fn workspace_variants_strip_known_prefixes_in_order() {
        let variants = workspace_name_variants(
            "agent-ws-ws-debug-auth",
            &[String::from("agent-ws"), String::from("codex-ws")],
        );
        assert_eq!(
            variants,
            vec![
                String::from("agent-ws-ws-debug-auth"),
                String::from("ws-debug-auth"),
                String::from("debug-auth"),
            ]
        );
    }

    #[test]
    fn create_name_normalization_keeps_plain_names() {
        assert_eq!(
            normalize_workspace_name_for_create("debug-auth"),
            String::from("debug-auth")
        );
    }

    #[test]
    fn create_translation_drops_no_extras_private_repo_and_extra_repos() {
        let translated = parse_create_args(&[
            OsString::from("--no-extras"),
            OsString::from("--private-repo"),
            OsString::from("org/private"),
            OsString::from("org/one"),
            OsString::from("org/two"),
        ])
        .expect("parse")
        .forwarded_args;
        let values: Vec<String> = translated
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(values, vec!["org/one"]);
    }

    #[test]
    fn create_parser_rejects_repo_args_with_no_work_repos() {
        let err = parse_create_args(&[
            OsString::from("--no-work-repos"),
            OsString::from("org/repo"),
        ])
        .expect_err("expected parse error");
        assert!(err.contains("--no-work-repos"));
    }

    #[test]
    fn exec_parser_extracts_user_workspace_and_command() {
        let parsed = parse_exec_args(&[
            OsString::from("--user"),
            OsString::from("codex"),
            OsString::from("ws-test"),
            OsString::from("id"),
            OsString::from("-u"),
        ])
        .expect("parse");

        assert!(!parsed.show_help);
        assert_eq!(parsed.user, Some(OsString::from("codex")));
        assert_eq!(parsed.workspace, Some(OsString::from("ws-test")));
        let command: Vec<String> = parsed
            .command
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(command, vec!["id", "-u"]);
    }

    #[test]
    fn exec_parser_supports_root_flag() {
        let parsed = parse_exec_args(&[
            OsString::from("--root"),
            OsString::from("ws-test"),
            OsString::from("id"),
            OsString::from("-u"),
        ])
        .expect("parse");
        assert_eq!(parsed.user, Some(OsString::from("0")));
        assert_eq!(parsed.workspace, Some(OsString::from("ws-test")));
    }

    #[test]
    fn rm_parser_accepts_yes_and_workspace() {
        let parsed =
            parse_rm_args(&[OsString::from("ws-test"), OsString::from("--yes")]).expect("parse");

        assert!(!parsed.all);
        assert_eq!(parsed.workspace, Some(OsString::from("ws-test")));
    }

    #[test]
    fn rm_parser_accepts_all() {
        let parsed =
            parse_rm_args(&[OsString::from("--all"), OsString::from("--yes")]).expect("parse");

        assert!(parsed.all);
        assert_eq!(parsed.workspace, None);
    }

    #[test]
    fn auth_parser_extracts_provider_and_container() {
        let parsed = parse_auth_args(&[
            OsString::from("github"),
            OsString::from("--host"),
            OsString::from("github.com"),
            OsString::from("ws-test"),
        ])
        .expect("parse");
        assert_eq!(parsed.provider.as_deref(), Some("github"));
        assert_eq!(parsed.host.as_deref(), Some("github.com"));
        assert_eq!(parsed.container.as_deref(), Some("ws-test"));
    }

    #[test]
    fn reset_repo_parser_extracts_ref_and_yes() {
        let parsed = parse_reset_repo_args(&[
            OsString::from("ws-test"),
            OsString::from("/work/org/repo"),
            OsString::from("--ref"),
            OsString::from("origin/dev"),
            OsString::from("--yes"),
        ])
        .expect("parse");
        assert_eq!(parsed.container.as_deref(), Some("ws-test"));
        assert_eq!(parsed.repo_dir.as_deref(), Some("/work/org/repo"));
        assert_eq!(parsed.refspec, "origin/dev");
        assert!(parsed.yes);
    }

    fn write_stub_launcher(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("launcher-stub.sh");
        fs::write(&path, launcher_script()).expect("write launcher stub");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("chmod");
        }

        path
    }

    fn launcher_script() -> &'static str {
        r#"#!/usr/bin/env bash
set -euo pipefail

log="${AW_TEST_LOG:-/dev/null}"
printf 'subcommand=%s\n' "$1" >"$log"
shift

i=0
for arg in "$@"; do
  printf 'arg%s=%s\n' "$i" "$arg" >>"$log"
  i=$((i + 1))
done

printf 'codex_secret_dir=%s\n' "${CODEX_SECRET_DIR:-}" >>"$log"
printf 'codex_auth_file=%s\n' "${CODEX_AUTH_FILE:-}" >>"$log"
exit "${AW_TEST_EXIT_CODE:-0}"
"#
    }
}
