mod auth;
mod container;
mod create;
mod exec;
mod ls;
mod reset;
mod rm;
mod tunnel;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::EXIT_RUNTIME;
use crate::runtime::{Runtime, resolve_runtime};

const PRIMARY_COMMAND_NAME: &str = "agent-workspace-launcher";
const WORKSPACE_META_FILE: &str = ".workspace-meta";

pub fn dispatch(subcommand: &str, args: &[OsString]) -> i32 {
    let (runtime, filtered_args) = match resolve_runtime(args) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!("hint: use --runtime container or --runtime host");
            return EXIT_RUNTIME;
        }
    };

    let status = match runtime {
        Runtime::Host => dispatch_host(subcommand, &filtered_args),
        Runtime::Container => dispatch_container(subcommand, &filtered_args),
    };

    if status != 0 && runtime == Runtime::Container && !command_exists("docker") {
        eprintln!(
            "hint: install/start Docker or retry with '--runtime host' (or AGENT_WORKSPACE_RUNTIME=host)"
        );
    }

    status
}

pub(crate) fn completion_workspace_names(runtime: Runtime) -> Result<Vec<String>, String> {
    match runtime {
        Runtime::Host => Ok(list_workspaces_on_disk()?
            .into_iter()
            .map(|workspace| workspace.name)
            .collect()),
        Runtime::Container => container::completion_workspace_names(),
    }
}

fn dispatch_host(subcommand: &str, args: &[OsString]) -> i32 {
    match subcommand {
        "auth" => auth::run(args),
        "create" => create::run(args),
        "ls" => ls::run(args),
        "rm" => rm::run(args),
        "exec" => exec::run(args),
        "reset" => reset::run(args),
        "tunnel" => tunnel::run(args),
        _ => {
            eprintln!("error: unknown subcommand: {subcommand}");
            EXIT_RUNTIME
        }
    }
}

fn dispatch_container(subcommand: &str, args: &[OsString]) -> i32 {
    container::dispatch(subcommand, args)
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
    dirs.push(String::from(
        "/home/agent/.config/zsh/scripts/_features/codex/secrets",
    ));
    dirs.push(String::from(
        "/home/codex/.config/zsh/scripts/_features/codex/secrets",
    ));
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use tempfile::TempDir;

    use super::{
        Workspace,
        auth::{codex_auth_targets, resolve_workspace_for_auth},
        create::parse_create_args,
        dispatch,
        exec::parse_exec_args,
        normalize_workspace_name_for_create, parse_repo_spec,
        reset::parse_reset_work_repos_args,
        resolve_codex_auth_file, resolve_codex_profile_auth_files,
        tunnel::parse_tunnel_args,
        workspace_name_variants, workspace_prefixes, workspace_storage_root,
    };

    fn with_workspace_env<T>(f: impl FnOnce(&TempDir) -> T) -> T {
        let _guard = crate::env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let temp = tempfile::tempdir().expect("tempdir");

        unsafe {
            std::env::set_var("AGENT_WORKSPACE_HOME", temp.path());
            std::env::set_var("AGENT_WORKSPACE_RUNTIME", "host");
            std::env::remove_var("AGENT_WORKSPACE_PREFIX");
            std::env::remove_var("CODEX_WORKSPACE_PREFIX");
        }

        let result = f(&temp);

        unsafe {
            std::env::remove_var("AGENT_WORKSPACE_HOME");
            std::env::remove_var("AGENT_WORKSPACE_RUNTIME");
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
        let _guard = crate::env_lock()
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
        let _guard = crate::env_lock()
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
