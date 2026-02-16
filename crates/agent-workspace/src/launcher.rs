use std::ffi::OsString;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::EXIT_RUNTIME;

const PRIMARY_COMMAND_NAME: &str = "agent-workspace-launcher";
const DEFAULT_REF: &str = "origin/main";
const WORKSPACE_META_FILE: &str = ".workspace-meta";

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

pub fn dispatch(subcommand: &str, args: &[OsString]) -> i32 {
    match subcommand {
        "auth" => run_auth(args),
        "create" => run_create(args),
        "ls" => run_ls(args),
        "rm" => run_rm(args),
        "exec" => run_exec(args),
        "reset" => run_reset(args),
        "tunnel" => run_tunnel(args),
        _ => {
            eprintln!("error: unknown subcommand: {subcommand}");
            EXIT_RUNTIME
        }
    }
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
    ignored_options: Vec<String>,
}

#[derive(Debug, Clone)]
struct RepoSpec {
    owner: String,
    repo: String,
    owner_repo: String,
    clone_url: String,
}

#[derive(Debug, Clone)]
struct Workspace {
    name: String,
    path: PathBuf,
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

fn run_create(args: &[OsString]) -> i32 {
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
            "warn: ignoring unsupported create options in host-native mode: {}",
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

    let root = match ensure_workspace_root() {
        Ok(root) => root,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let workspace_path = root.join(&workspace_name);
    if workspace_path.exists() {
        eprintln!("error: workspace already exists: {workspace_name}");
        return EXIT_RUNTIME;
    }

    if let Err(err) =
        create_workspace_skeleton(&workspace_path, &workspace_name, primary_spec.as_ref())
    {
        eprintln!("error: {err}");
        return EXIT_RUNTIME;
    }

    if !parsed.no_work_repos
        && let Some(spec) = primary_spec.as_ref()
    {
        let destination = workspace_repo_destination(&workspace_path.join("work"), spec);
        if let Err(err) = clone_repo_into(spec, &destination) {
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
                let destination =
                    workspace_repo_destination(&workspace_path.join("private"), &spec);
                if let Err(err) = clone_repo_into(&spec, &destination) {
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
                let destination = workspace_repo_destination(&workspace_path.join("work"), &spec);
                if let Err(err) = clone_repo_into(&spec, &destination) {
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

    println!("workspace: {workspace_name}");
    println!("path: {}", workspace_path.display());
    0
}

fn create_workspace_skeleton(
    workspace_path: &Path,
    workspace_name: &str,
    primary_repo: Option<&RepoSpec>,
) -> Result<(), String> {
    fs::create_dir_all(workspace_path).map_err(|err| {
        format!(
            "failed to create workspace directory {}: {err}",
            workspace_path.display()
        )
    })?;

    for subdir in ["work", "opt", "private", "auth", ".codex"] {
        fs::create_dir_all(workspace_path.join(subdir)).map_err(|err| {
            format!(
                "failed to create workspace subdir {}: {err}",
                workspace_path.join(subdir).display()
            )
        })?;
    }

    let created_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    let metadata = format!(
        "name={workspace_name}\ncreated_unix={created_unix}\nprimary_repo={}\n",
        primary_repo
            .map(|repo| repo.owner_repo.as_str())
            .unwrap_or("none")
    );
    fs::write(workspace_path.join(WORKSPACE_META_FILE), metadata).map_err(|err| {
        format!(
            "failed to write workspace metadata {}: {err}",
            workspace_path.join(WORKSPACE_META_FILE).display()
        )
    })?;

    Ok(())
}

fn clone_repo_into(repo: &RepoSpec, destination: &Path) -> Result<(), String> {
    if destination.join(".git").is_dir() {
        return Ok(());
    }

    if destination.exists() {
        return Err(format!(
            "destination exists but is not a git repo: {}",
            destination.display()
        ));
    }

    if !command_exists("git") {
        return Err(String::from("git not found in PATH"));
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create clone parent {}: {err}", parent.display()))?;
    }

    let status = Command::new("git")
        .arg("clone")
        .arg("--progress")
        .arg(&repo.clone_url)
        .arg(destination)
        .status()
        .map_err(|err| format!("failed to run git clone for {}: {err}", repo.owner_repo))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "git clone failed for {} (exit {})",
            repo.owner_repo,
            status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

#[derive(Debug, Default, Clone)]
struct ParsedLs {
    show_help: bool,
    json: bool,
}

fn parse_ls_args(args: &[OsString]) -> Result<ParsedLs, String> {
    let mut parsed = ParsedLs::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let arg = args[idx].to_string_lossy();
        match arg.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--json" => parsed.json = true,
            "--output" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --output"));
                }
                let output = args[idx].to_string_lossy();
                if output != "json" {
                    return Err(format!("unsupported --output value: {output}"));
                }
                parsed.json = true;
            }
            _ if arg.starts_with("--output=") => {
                let output = &arg["--output=".len()..];
                if output != "json" {
                    return Err(format!("unsupported --output value: {output}"));
                }
                parsed.json = true;
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option for ls: {arg}")),
            _ => return Err(format!("unexpected arg for ls: {arg}")),
        }
        idx += 1;
    }

    Ok(parsed)
}

fn run_ls(args: &[OsString]) -> i32 {
    let parsed = match parse_ls_args(args) {
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

    let workspaces = match list_workspaces_on_disk() {
        Ok(workspaces) => workspaces,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if parsed.json {
        print_workspaces_json(&workspaces);
    } else {
        for workspace in workspaces {
            println!("{}", workspace.name);
        }
    }

    0
}

fn print_workspaces_json(workspaces: &[Workspace]) {
    let mut out = String::from("{\"workspaces\":[");
    for (idx, workspace) in workspaces.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"path\":\"{}\"}}",
            json_escape(&workspace.name),
            json_escape(&workspace.path.to_string_lossy())
        ));
    }
    out.push_str("]}");
    println!("{out}");
}

#[derive(Debug, Default, Clone)]
struct ParsedRm {
    show_help: bool,
    all: bool,
    yes: bool,
    workspace: Option<String>,
}

fn parse_rm_args(args: &[OsString]) -> Result<ParsedRm, String> {
    let mut parsed = ParsedRm::default();

    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--all" => parsed.all = true,
            "-y" | "--yes" => parsed.yes = true,
            _ if text.starts_with('-') => {
                return Err(format!("unknown option for rm: {text}"));
            }
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

    if parsed.all && parsed.workspace.is_some() {
        eprintln!("error: rm --all does not accept a workspace name");
        print_rm_usage();
        return EXIT_RUNTIME;
    }

    let targets = if parsed.all {
        match list_workspaces_on_disk() {
            Ok(items) => items,
            Err(err) => {
                eprintln!("error: {err}");
                return EXIT_RUNTIME;
            }
        }
    } else if let Some(workspace_name) = parsed.workspace.as_deref() {
        match resolve_workspace(workspace_name) {
            Ok(Some(workspace)) => vec![workspace],
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
            println!("  - {}", target.name);
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    for target in targets {
        if let Err(err) = fs::remove_dir_all(&target.path) {
            eprintln!(
                "error: failed to remove workspace {} ({}): {err}",
                target.name,
                target.path.display()
            );
            return EXIT_RUNTIME;
        }
        println!("removed: {}", target.name);
    }

    0
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

    let workspace_name = parsed
        .workspace
        .as_ref()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if parsed.user.is_some() {
        eprintln!("warn: --root/--user is ignored in host-native exec mode");
    }

    let mut command = if parsed.command.is_empty() {
        let shell = std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/bash"));
        let mut cmd = Command::new(shell);
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            cmd.arg("-l");
        }
        cmd
    } else {
        let mut cmd = Command::new(&parsed.command[0]);
        if parsed.command.len() > 1 {
            cmd.args(&parsed.command[1..]);
        }
        cmd
    };

    command.current_dir(&workspace.path);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    match command.status() {
        Ok(status) => status.code().unwrap_or(EXIT_RUNTIME),
        Err(err) => {
            eprintln!(
                "error: failed to run command in {}: {err}",
                workspace.path.display()
            );
            EXIT_RUNTIME
        }
    }
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
            "--" => {
                idx += 1;
                while idx < args.len() {
                    let text = args[idx].to_string_lossy().into_owned();
                    if parsed.provider.is_none() {
                        parsed.provider = Some(text);
                    } else if parsed.workspace.is_none() {
                        parsed.workspace = Some(text);
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

    let workspace = match resolve_workspace_for_auth(parsed.workspace.as_deref()) {
        Ok(workspace) => workspace,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let provider = parsed
        .provider
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    match provider.as_str() {
        "github" => run_auth_github(&workspace, parsed.host.as_deref()),
        "codex" => run_auth_codex(&workspace, parsed.profile.as_deref()),
        "gpg" => run_auth_gpg(&workspace, parsed.key.as_deref()),
        _ => {
            eprintln!("error: unknown auth provider: {provider}");
            eprintln!("hint: expected: codex|github|gpg");
            EXIT_RUNTIME
        }
    }
}

fn run_auth_github(workspace: &Workspace, host: Option<&str>) -> i32 {
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
    let target = workspace.path.join("auth").join("github.env");
    if let Err(err) = write_file_secure(&target, content.as_bytes()) {
        eprintln!(
            "error: failed to write GitHub auth file {}: {err}",
            target.display()
        );
        return EXIT_RUNTIME;
    }

    println!(
        "auth: github -> {} ({gh_host}; source={chosen_source})",
        workspace.name
    );
    0
}

fn run_auth_codex(workspace: &Workspace, profile_arg: Option<&str>) -> i32 {
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

    let mut candidate_files: Vec<PathBuf> = Vec::new();
    if let Some(profile) = profile.as_deref() {
        if profile.contains('/')
            || profile.contains("..")
            || profile.chars().any(char::is_whitespace)
        {
            eprintln!("error: invalid codex profile name: {profile}");
            return EXIT_RUNTIME;
        }

        for candidate in resolve_codex_profile_auth_files(profile) {
            push_unique_path(&mut candidate_files, PathBuf::from(candidate));
        }
    }
    push_unique_path(
        &mut candidate_files,
        PathBuf::from(resolve_codex_auth_file()),
    );

    for candidate in candidate_files {
        if !candidate.is_file() {
            continue;
        }

        let auth_data = match fs::read(&candidate) {
            Ok(data) => data,
            Err(err) => {
                eprintln!(
                    "warn: failed to read codex auth candidate {}: {err}",
                    candidate.display()
                );
                continue;
            }
        };

        if let Err(err) = sync_codex_auth_into_workspace(workspace, &auth_data) {
            eprintln!(
                "warn: failed to sync codex auth from {}: {err}",
                candidate.display()
            );
            continue;
        }

        if let Some(profile) = profile.as_deref() {
            println!(
                "auth: codex -> {} (profile={profile}; source={})",
                workspace.name,
                candidate.display()
            );
        } else {
            println!(
                "auth: codex -> {} (source={})",
                workspace.name,
                candidate.display()
            );
        }
        return 0;
    }

    eprintln!("error: unable to resolve codex auth file");
    eprintln!("hint: set CODEX_AUTH_FILE or pass --profile <name>");
    EXIT_RUNTIME
}

fn sync_codex_auth_into_workspace(workspace: &Workspace, auth_data: &[u8]) -> Result<(), String> {
    let targets = codex_auth_targets(workspace);
    for target in targets {
        write_file_secure(&target, auth_data)?;
    }
    Ok(())
}

fn run_auth_gpg(workspace: &Workspace, key_arg: Option<&str>) -> i32 {
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

    if command_exists("gpg") {
        let status = Command::new("gpg")
            .args(["--batch", "--list-secret-keys", &key])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match status {
            Ok(result) if result.success() => {}
            Ok(_) => {
                eprintln!("error: gpg key not found in host keyring: {key}");
                return EXIT_RUNTIME;
            }
            Err(err) => {
                eprintln!("error: failed to run gpg for key lookup: {err}");
                return EXIT_RUNTIME;
            }
        }
    } else {
        eprintln!("warn: gpg not found in PATH; writing key id only");
    }

    let target = workspace.path.join("auth").join("gpg-key.txt");
    if let Err(err) = write_file_secure(&target, format!("{key}\n").as_bytes()) {
        eprintln!(
            "error: failed to write gpg auth file {}: {err}",
            target.display()
        );
        return EXIT_RUNTIME;
    }

    println!("auth: gpg -> {} (key={key})", workspace.name);
    0
}

fn codex_auth_targets(workspace: &Workspace) -> Vec<PathBuf> {
    let mut targets = vec![workspace.path.join(".codex").join("auth.json")];

    if let Ok(value) = std::env::var("CODEX_AUTH_FILE")
        && let Some(cleaned) = trimmed_nonempty(&value)
    {
        let mapped = map_workspace_internal_path(workspace, &cleaned);
        push_unique_path(&mut targets, mapped);
    }

    targets
}

fn map_workspace_internal_path(workspace: &Workspace, raw: &str) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        let trimmed = raw.trim_start_matches('/');
        if trimmed.is_empty() {
            workspace.path.join(".codex").join("auth.json")
        } else {
            workspace.path.join(trimmed)
        }
    } else {
        workspace.path.join(path)
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

fn resolve_workspace_for_auth(name: Option<&str>) -> Result<Workspace, String> {
    if let Some(name) = name.and_then(trimmed_nonempty) {
        return match resolve_workspace(&name)? {
            Some(workspace) => Ok(workspace),
            None => Err(format!("workspace not found: {name}")),
        };
    }

    let workspaces = list_workspaces_on_disk()?;
    match workspaces.as_slice() {
        [] => Err(String::from("no workspaces found")),
        [single] => Ok(single.clone()),
        _ => Err(format!(
            "multiple workspaces found; specify one: {}",
            workspaces
                .iter()
                .map(|workspace| workspace.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
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

    let workspace_name = if let Some(workspace) = parsed.workspace {
        workspace
    } else {
        eprintln!("error: missing workspace");
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

    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let target_repo = map_workspace_repo_path(&workspace, &repo_dir);
    if !target_repo.join(".git").exists() {
        eprintln!("error: not a git repo: {}", target_repo.display());
        return EXIT_RUNTIME;
    }

    if !parsed.yes {
        println!("This will reset a repo in workspace: {}", workspace.name);
        println!("  - {}", target_repo.display());
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    match reset_repo_on_host(&target_repo, &parsed.refspec) {
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

    let workspace_name = if let Some(workspace) = parsed.workspace {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_work_repos_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let root = map_workspace_repo_path(&workspace, &parsed.root);
    let repos = match list_git_repos_on_host(&root, parsed.depth) {
        Ok(repos) => repos,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if repos.is_empty() {
        eprintln!(
            "warn: no git repos found under {} (depth={}) in {}",
            root.display(),
            parsed.depth,
            workspace.name
        );
        return 0;
    }

    if !parsed.yes {
        println!(
            "This will reset {} repo(s) inside workspace: {}",
            repos.len(),
            workspace.name
        );
        for repo in &repos {
            println!("  - {}", repo.display());
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    let mut failed = 0usize;
    for repo in repos {
        if let Err(err) = reset_repo_on_host(&repo, &parsed.refspec) {
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

    let workspace_name = if let Some(workspace) = parsed.workspace {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_opt_repos_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let opt_root = workspace.path.join("opt");
    let repos = match list_git_repos_on_host(&opt_root, 4) {
        Ok(repos) => repos,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if repos.is_empty() {
        eprintln!("warn: no git repos found under {}", opt_root.display());
        return 0;
    }

    if !parsed.yes {
        println!(
            "This will reset /opt-style repos in workspace: {}",
            workspace.name
        );
        for repo in &repos {
            println!("  - {}", repo.display());
        }
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    for repo in repos {
        if let Err(err) = reset_repo_on_host(&repo, DEFAULT_REF) {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    }

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

    let workspace_name = if let Some(workspace) = parsed.workspace {
        workspace
    } else {
        eprintln!("error: missing workspace");
        print_reset_private_repo_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(&workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    let private_repo = match detect_private_repo_dir(&workspace) {
        Ok(Some(path)) => path,
        Ok(None) => {
            eprintln!(
                "warn: no private git repo found in workspace: {}",
                workspace.name
            );
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
        println!(
            "This will reset private repo in workspace: {}",
            workspace.name
        );
        println!("  - {}", private_repo.display());
        if !confirm_or_abort("Proceed? [y/N] ") {
            println!("Aborted");
            return EXIT_RUNTIME;
        }
    }

    match reset_repo_on_host(&private_repo, &parsed.refspec) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("error: {err}");
            EXIT_RUNTIME
        }
    }
}

fn detect_private_repo_dir(workspace: &Workspace) -> Result<Option<PathBuf>, String> {
    let private_root = workspace.path.join("private");
    if !private_root.exists() {
        return Ok(None);
    }

    let repos = list_git_repos_on_host(&private_root, 4)?;
    Ok(repos.into_iter().next())
}

fn reset_repo_on_host(repo_dir: &Path, refspec: &str) -> Result<(), String> {
    let status = Command::new("bash")
        .args([
            "-c",
            RESET_REPO_SCRIPT,
            "--",
            repo_dir.to_string_lossy().as_ref(),
            refspec,
        ])
        .status()
        .map_err(|err| format!("failed to reset repo {}: {err}", repo_dir.display()))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "failed to reset repo {} (exit {})",
            repo_dir.display(),
            status.code().unwrap_or(EXIT_RUNTIME)
        ))
    }
}

fn list_git_repos_on_host(root: &Path, depth: u32) -> Result<Vec<PathBuf>, String> {
    if depth == 0 {
        return Err(String::from("--depth must be a positive integer"));
    }

    let output = Command::new("bash")
        .args([
            "-c",
            LIST_GIT_REPOS_SCRIPT,
            "--",
            root.to_string_lossy().as_ref(),
            &depth.to_string(),
        ])
        .output()
        .map_err(|err| format!("failed to list git repos under {}: {err}", root.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "failed to list git repos under {} (exit {}): {stderr}",
            root.display(),
            output.status.code().unwrap_or(EXIT_RUNTIME)
        ));
    }

    let mut repos: Vec<PathBuf> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect();
    repos.sort();
    repos.dedup();
    Ok(repos)
}

fn map_workspace_repo_path(workspace: &Workspace, raw: &str) -> PathBuf {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return workspace.path.clone();
    }

    if cleaned == "/work" {
        return workspace.path.join("work");
    }
    if let Some(rest) = cleaned.strip_prefix("/work/") {
        return workspace.path.join("work").join(rest);
    }

    if cleaned == "/opt" {
        return workspace.path.join("opt");
    }
    if let Some(rest) = cleaned.strip_prefix("/opt/") {
        return workspace.path.join("opt").join(rest);
    }

    if cleaned == "~/.private" {
        return workspace.path.join("private");
    }
    if let Some(rest) = cleaned.strip_prefix("~/.private/") {
        return workspace.path.join("private").join(rest);
    }

    let path = Path::new(cleaned);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.path.join(path)
    }
}

#[derive(Debug, Clone)]
struct ParsedResetRepo {
    show_help: bool,
    workspace: Option<String>,
    repo_dir: Option<String>,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetRepo {
    fn default() -> Self {
        Self {
            show_help: false,
            workspace: None,
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
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(text.to_string());
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
    workspace: Option<String>,
    root: String,
    depth: u32,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetWorkRepos {
    fn default() -> Self {
        Self {
            show_help: false,
            workspace: None,
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
            _ if text.starts_with("--root=") => {
                parsed.root = text["--root=".len()..].to_string();
            }
            _ if text.starts_with("--depth=") => {
                parsed.depth = text["--depth=".len()..]
                    .parse::<u32>()
                    .map_err(|_| String::from("--depth must be a positive integer"))?;
            }
            _ if text.starts_with("--ref=") => {
                parsed.refspec = text["--ref=".len()..].to_string();
            }
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(text.to_string());
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
    workspace: Option<String>,
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
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(text.to_string());
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
    workspace: Option<String>,
    refspec: String,
    yes: bool,
}

impl Default for ParsedResetPrivate {
    fn default() -> Self {
        Self {
            show_help: false,
            workspace: None,
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
            _ if text.starts_with("--ref=") => {
                parsed.refspec = text["--ref=".len()..].to_string();
            }
            _ if text.starts_with('-') => return Err(format!("unknown arg: {text}")),
            _ => {
                if parsed.workspace.is_none() {
                    parsed.workspace = Some(text.to_string());
                } else {
                    return Err(format!("unexpected arg: {text}"));
                }
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

#[derive(Debug, Default, Clone)]
struct ParsedTunnel {
    show_help: bool,
    workspace: Option<String>,
    tunnel_name: Option<String>,
    detach: bool,
    output_json: bool,
}

fn parse_tunnel_args(args: &[OsString]) -> Result<ParsedTunnel, String> {
    let mut parsed = ParsedTunnel::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "-h" | "--help" => parsed.show_help = true,
            "--detach" => parsed.detach = true,
            "--name" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --name"));
                }
                parsed.tunnel_name = trimmed_nonempty(args[idx].to_string_lossy().as_ref());
            }
            "--output" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --output"));
                }
                let value = args[idx].to_string_lossy();
                if value != "json" {
                    return Err(format!("unsupported --output value: {value}"));
                }
                parsed.output_json = true;
            }
            _ if current.starts_with("--name=") => {
                parsed.tunnel_name = trimmed_nonempty(&current["--name=".len()..]);
            }
            _ if current.starts_with("--output=") => {
                let value = &current["--output=".len()..];
                if value != "json" {
                    return Err(format!("unsupported --output value: {value}"));
                }
                parsed.output_json = true;
            }
            _ if current.starts_with('-') => {
                return Err(format!("unknown option for tunnel: {current}"));
            }
            _ => {
                if parsed.workspace.is_some() {
                    return Err(format!("unexpected arg for tunnel: {current}"));
                }
                parsed.workspace = Some(current.to_string());
            }
        }
        idx += 1;
    }

    Ok(parsed)
}

fn run_tunnel(args: &[OsString]) -> i32 {
    let parsed = match parse_tunnel_args(args) {
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

    let workspace_name = if let Some(workspace_name) = parsed.workspace.as_deref() {
        workspace_name
    } else {
        eprintln!("error: missing workspace name");
        print_tunnel_usage();
        return EXIT_RUNTIME;
    };

    let workspace = match resolve_workspace(workspace_name) {
        Ok(Some(workspace)) => workspace,
        Ok(None) => {
            eprintln!("error: workspace not found: {workspace_name}");
            return EXIT_RUNTIME;
        }
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if !command_exists("code") {
        eprintln!("error: 'code' command not found in PATH (required for tunnel)");
        return EXIT_RUNTIME;
    }

    let mut cmd = Command::new("code");
    cmd.arg("tunnel");
    cmd.arg("--accept-server-license-terms");
    if let Some(tunnel_name) = parsed.tunnel_name.as_deref() {
        cmd.args(["--name", tunnel_name]);
    }
    cmd.current_dir(&workspace.path);

    if parsed.detach {
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        return match cmd.spawn() {
            Ok(child) => {
                if parsed.output_json {
                    println!(
                        "{{\"workspace\":\"{}\",\"detached\":true,\"pid\":{}}}",
                        json_escape(&workspace.name),
                        child.id()
                    );
                } else {
                    println!("tunnel: {} detached (pid={})", workspace.name, child.id());
                }
                0
            }
            Err(err) => {
                eprintln!("error: failed to launch tunnel: {err}");
                EXIT_RUNTIME
            }
        };
    }

    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => {
            let code = status.code().unwrap_or(EXIT_RUNTIME);
            if parsed.output_json {
                println!(
                    "{{\"workspace\":\"{}\",\"detached\":false,\"exit_code\":{}}}",
                    json_escape(&workspace.name),
                    code
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

fn workspace_repo_destination(root: &Path, repo: &RepoSpec) -> PathBuf {
    root.join(&repo.owner).join(&repo.repo)
}

fn list_workspaces_on_disk() -> Result<Vec<Workspace>, String> {
    let root = ensure_workspace_root()?;

    let mut workspaces: Vec<Workspace> = Vec::new();
    for entry in fs::read_dir(&root)
        .map_err(|err| format!("failed to read workspace root {}: {err}", root.display()))?
    {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read workspace directory entry under {}: {err}",
                root.display()
            )
        })?;

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };

        workspaces.push(Workspace { name, path });
    }

    workspaces.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(workspaces)
}

fn resolve_workspace(name: &str) -> Result<Option<Workspace>, String> {
    let workspace_name = match trimmed_nonempty(name) {
        Some(name) => name,
        None => return Ok(None),
    };

    let root = ensure_workspace_root()?;
    let prefixes = workspace_prefixes();

    for candidate in workspace_resolution_candidates(&workspace_name, &prefixes) {
        let path = root.join(&candidate);
        if path.is_dir() {
            return Ok(Some(Workspace {
                name: candidate,
                path,
            }));
        }
    }

    Ok(None)
}

fn ensure_workspace_root() -> Result<PathBuf, String> {
    let root = workspace_storage_root();
    fs::create_dir_all(&root)
        .map_err(|err| format!("failed to create workspace root {}: {err}", root.display()))?;
    Ok(root)
}

fn workspace_storage_root() -> PathBuf {
    if let Ok(value) = std::env::var("AGENT_WORKSPACE_HOME")
        && let Some(cleaned) = trimmed_nonempty(&value)
    {
        return PathBuf::from(cleaned);
    }

    if let Ok(value) = std::env::var("XDG_STATE_HOME")
        && let Some(cleaned) = trimmed_nonempty(&value)
    {
        return PathBuf::from(cleaned)
            .join("agent-workspace-launcher")
            .join("workspaces");
    }

    if let Ok(home) = std::env::var("HOME")
        && let Some(cleaned) = trimmed_nonempty(&home)
    {
        return PathBuf::from(cleaned)
            .join(".local")
            .join("state")
            .join("agent-workspace-launcher")
            .join("workspaces");
    }

    std::env::temp_dir()
        .join("agent-workspace-launcher")
        .join("workspaces")
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

fn workspace_resolution_candidates(workspace_name: &str, prefixes: &[String]) -> Vec<String> {
    let variants = workspace_name_variants(workspace_name, prefixes);
    let mut candidates: Vec<String> = Vec::new();

    for variant in &variants {
        push_unique(&mut candidates, variant.clone());
    }

    for variant in variants {
        for prefix in prefixes {
            let prefixed = if variant.starts_with(&(prefix.clone() + "-")) {
                variant.clone()
            } else {
                format!("{prefix}-{variant}")
            };
            push_unique(&mut candidates, prefixed);
        }
    }

    candidates
}

fn normalize_workspace_name_for_create(name: &str) -> String {
    let variants = workspace_name_variants(name, &workspace_prefixes());
    let resolved = variants
        .last()
        .cloned()
        .or_else(|| trimmed_nonempty(name))
        .unwrap_or_else(|| String::from("workspace"));
    slugify_name(&resolved)
}

fn slugify_name(name: &str) -> String {
    let mut out = String::new();

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else if (ch.is_ascii_whitespace() || matches!(ch, '/' | '.' | ':')) && !out.ends_with('-')
        {
            out.push('-');
        }
    }

    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        String::from("workspace")
    } else {
        out
    }
}

fn generate_workspace_name() -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("ws-{suffix}")
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

fn write_file_secure(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create parent {}: {err}", parent.display()))?;
    }

    fs::write(path, contents)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    set_owner_only_permissions(path);
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) {}

fn confirm_or_abort(prompt: &str) -> bool {
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn trimmed_nonempty(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|known| known == &value) {
        values.push(value);
    }
}

fn push_unique_path(values: &mut Vec<PathBuf>, value: PathBuf) {
    if !values.iter().any(|known| known == &value) {
        values.push(value);
    }
}

fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn print_create_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} create [--name <workspace>] [--private-repo OWNER/REPO] [--no-work-repos] [--no-extras] [repo] [extra_repos...]"
    );
}

fn print_ls_usage() {
    eprintln!("usage: {PRIMARY_COMMAND_NAME} ls [--json|--output json]");
}

fn print_exec_usage() {
    eprintln!(
        "usage: {PRIMARY_COMMAND_NAME} exec [--root|--user <user>] <workspace> [command ...]"
    );
}

fn print_rm_usage() {
    eprintln!("usage: {PRIMARY_COMMAND_NAME} rm [--all] [--yes] <workspace>");
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
        "  {PRIMARY_COMMAND_NAME} reset work-repos <workspace> [--root <dir>] [--depth <N>] [--ref <remote/branch>] [--yes]"
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
        "usage: {PRIMARY_COMMAND_NAME} reset work-repos <workspace> [--root <dir>] [--depth <N>] [--ref <remote/branch>] [--yes]"
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

fn print_tunnel_usage() {
    println!("usage:");
    println!(
        "  {PRIMARY_COMMAND_NAME} tunnel <workspace> [--name <tunnel_name>] [--detach] [--output json]"
    );
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};

    use tempfile::TempDir;

    use super::{
        Workspace, codex_auth_targets, dispatch, normalize_workspace_name_for_create,
        parse_create_args, parse_exec_args, parse_repo_spec, parse_reset_work_repos_args,
        parse_tunnel_args, resolve_codex_auth_file, resolve_codex_profile_auth_files,
        resolve_workspace_for_auth, workspace_name_variants, workspace_prefixes,
        workspace_storage_root,
    };

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_workspace_env<T>(f: impl FnOnce(&TempDir) -> T) -> T {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let temp = tempfile::tempdir().expect("tempdir");

        unsafe {
            std::env::set_var("AGENT_WORKSPACE_HOME", temp.path());
            std::env::remove_var("AGENT_WORKSPACE_PREFIX");
            std::env::remove_var("CODEX_WORKSPACE_PREFIX");
        }

        let result = f(&temp);

        unsafe {
            std::env::remove_var("AGENT_WORKSPACE_HOME");
            std::env::remove_var("AGENT_WORKSPACE_PREFIX");
            std::env::remove_var("CODEX_WORKSPACE_PREFIX");
            std::env::remove_var("CODEX_AUTH_FILE");
            std::env::remove_var("CODEX_SECRET_DIR");
            std::env::remove_var("XDG_STATE_HOME");
            std::env::remove_var("HOME");
        }

        result
    }

    #[test]
    fn parse_repo_spec_accepts_owner_repo() {
        let parsed = parse_repo_spec("octo/demo", "github.com").expect("parse owner/repo");
        assert_eq!(parsed.owner, "octo");
        assert_eq!(parsed.repo, "demo");
        assert_eq!(parsed.owner_repo, "octo/demo");
        assert_eq!(parsed.clone_url, "https://github.com/octo/demo.git");
    }

    #[test]
    fn parse_repo_spec_accepts_https_url() {
        let parsed = parse_repo_spec("https://example.com/octo/demo.git", "github.com")
            .expect("parse https url");
        assert_eq!(parsed.owner_repo, "octo/demo");
        assert_eq!(parsed.clone_url, "https://example.com/octo/demo.git");
    }

    #[test]
    fn workspace_variants_strip_prefixes() {
        let prefixes = workspace_prefixes();
        let variants = workspace_name_variants("agent-ws-ws-demo", &prefixes);
        assert_eq!(variants, vec!["agent-ws-ws-demo", "ws-demo", "demo"]);
    }

    #[test]
    fn normalize_workspace_name_for_create_strips_prefixes() {
        assert_eq!(
            normalize_workspace_name_for_create("agent-ws-ws-demo"),
            "demo"
        );
    }

    #[test]
    fn parse_create_rejects_repos_with_no_work_repos() {
        let err = parse_create_args(&[
            OsString::from("--no-work-repos"),
            OsString::from("octo/demo"),
        ])
        .expect_err("reject repos with --no-work-repos");
        assert!(err.contains("--no-work-repos"));
    }

    #[test]
    fn parse_exec_supports_user_and_command() {
        let parsed = parse_exec_args(&[
            OsString::from("--user"),
            OsString::from("agent"),
            OsString::from("ws-test"),
            OsString::from("git"),
            OsString::from("status"),
        ])
        .expect("parse exec args");

        assert_eq!(
            parsed
                .workspace
                .as_ref()
                .map(|value| value.to_string_lossy().into_owned())
                .as_deref(),
            Some("ws-test")
        );
        assert_eq!(
            parsed
                .user
                .as_ref()
                .map(|value| value.to_string_lossy().into_owned())
                .as_deref(),
            Some("agent")
        );
        assert_eq!(
            parsed
                .command
                .iter()
                .map(|value| value.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec!["git", "status"]
        );
    }

    #[test]
    fn parse_reset_work_repos_rejects_depth_zero() {
        let err = parse_reset_work_repos_args(&[
            OsString::from("ws-test"),
            OsString::from("--depth"),
            OsString::from("0"),
        ])
        .expect_err("reject depth zero");
        assert!(err.contains("positive integer"));
    }

    #[test]
    fn parse_tunnel_supports_output_json_and_detach() {
        let parsed = parse_tunnel_args(&[
            OsString::from("ws-test"),
            OsString::from("--detach"),
            OsString::from("--output"),
            OsString::from("json"),
        ])
        .expect("parse tunnel args");

        assert_eq!(parsed.workspace.as_deref(), Some("ws-test"));
        assert!(parsed.detach);
        assert!(parsed.output_json);
    }

    #[test]
    fn workspace_storage_root_uses_explicit_env() {
        with_workspace_env(|temp| {
            let root = workspace_storage_root();
            assert_eq!(root, temp.path());
        });
    }

    #[test]
    fn create_ls_rm_lifecycle_works_without_repos() {
        with_workspace_env(|temp| {
            let code = dispatch(
                "create",
                &[
                    OsString::from("--no-work-repos"),
                    OsString::from("--name"),
                    OsString::from("ws-test"),
                ],
            );
            assert_eq!(code, 0);
            assert!(temp.path().join("test").is_dir());

            let remove_code = dispatch("rm", &[OsString::from("--yes"), OsString::from("test")]);
            assert_eq!(remove_code, 0);
            assert!(!temp.path().join("test").exists());
        });
    }

    #[test]
    fn resolve_workspace_for_auth_uses_single_workspace_when_unspecified() {
        with_workspace_env(|temp| {
            std::fs::create_dir_all(temp.path().join("ws-only")).expect("create workspace");

            let workspace = resolve_workspace_for_auth(None).expect("resolve default workspace");
            assert_eq!(workspace.name, "ws-only");
        });
    }

    #[test]
    fn codex_auth_targets_include_compat_path() {
        with_workspace_env(|temp| {
            unsafe {
                std::env::set_var("CODEX_AUTH_FILE", "/home/agent/.codex/auth.json");
            }
            let workspace = Workspace {
                name: String::from("ws-test"),
                path: temp.path().join("ws-test"),
            };

            let targets = codex_auth_targets(&workspace);
            assert!(
                targets
                    .iter()
                    .any(|path| path.ends_with(".codex/auth.json"))
            );
            assert!(
                targets
                    .iter()
                    .any(|path| path.ends_with("home/agent/.codex/auth.json"))
            );
        });
    }

    #[test]
    fn resolve_codex_auth_file_prefers_env() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        unsafe {
            std::env::set_var("CODEX_AUTH_FILE", "/tmp/custom-auth.json");
        }
        assert_eq!(resolve_codex_auth_file(), "/tmp/custom-auth.json");
        unsafe {
            std::env::remove_var("CODEX_AUTH_FILE");
        }
    }

    #[test]
    fn resolve_codex_profile_auth_files_prefers_secret_dir() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        unsafe {
            std::env::set_var("CODEX_SECRET_DIR", "/tmp/secrets");
        }

        let files = resolve_codex_profile_auth_files("work");
        assert!(files.iter().any(|path| path == "/tmp/secrets/work.json"));
        assert!(files.iter().any(|path| path == "/tmp/secrets/work"));

        unsafe {
            std::env::remove_var("CODEX_SECRET_DIR");
        }
    }
}
