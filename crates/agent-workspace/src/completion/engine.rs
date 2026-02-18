use crate::runtime::Runtime;

use super::candidates::{
    Candidate, finalize, push_described_values, push_global_options, push_values, with_prefix,
};
use super::protocol::CompletionRequest;

pub(crate) trait WorkspaceProvider {
    fn list_workspaces(&self, runtime: Runtime) -> Result<Vec<String>, String>;
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CompletionResult {
    pub(crate) candidates: Vec<Candidate>,
    pub(crate) workspace_error: Option<String>,
}

pub(crate) fn complete<P: WorkspaceProvider>(
    request: &CompletionRequest,
    provider: &P,
) -> CompletionResult {
    let current = request.current_word();
    let words_before = request.words_before_cursor();

    let mut workspace_ctx = WorkspaceContext::new(provider, request.runtime);

    let candidates = if let Some(runtime_candidates) = complete_runtime_value(words_before, current)
    {
        runtime_candidates
    } else if let Some((subcommand_idx, subcommand)) = detect_subcommand(words_before) {
        let args_before = strip_runtime_tokens(&words_before[(subcommand_idx + 1)..]);
        match subcommand {
            "auth" => complete_auth(current, &args_before, &mut workspace_ctx),
            "create" => complete_create(current, &args_before),
            "ls" => complete_ls(current, &args_before),
            "rm" => complete_rm(&args_before, &mut workspace_ctx),
            "exec" => complete_exec(current, &args_before, &mut workspace_ctx),
            "reset" => complete_reset(current, &args_before, &mut workspace_ctx),
            "tunnel" => complete_tunnel(current, &args_before, &mut workspace_ctx),
            _ => Vec::new(),
        }
    } else {
        complete_top_level(current)
    };

    CompletionResult {
        candidates: finalize(candidates, current),
        workspace_error: workspace_ctx.workspace_error,
    }
}

fn complete_top_level(_current: &str) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("auth", "Update auth material in workspace"),
            ("create", "Create a new workspace"),
            ("ls", "List workspaces"),
            ("rm", "Remove workspace(s)"),
            ("exec", "Run command in workspace"),
            ("reset", "Reset repos in workspace"),
            ("tunnel", "Start VS Code tunnel"),
        ],
    );
    push_global_options(&mut out);
    out
}

fn complete_create(current: &str, args_before: &[String]) -> Vec<Candidate> {
    if let Some((option, inline)) = value_option(
        args_before,
        current,
        &["--name", "--image", "--ref", "--private-repo"],
    ) {
        return value_suggestions(&option, inline, &[]);
    }

    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("--name", "Set workspace name"),
            ("--image", "Use custom agent image"),
            ("--ref", "Pin work repos to git ref"),
            ("--private-repo", "Attach private repository"),
            ("--no-work-repos", "Skip cloning work repositories"),
            ("--no-extras", "Skip optional setup extras"),
            ("--no-pull", "Do not pull image before create"),
            ("--help", "Show help for create"),
            ("-h", "Show help for create"),
        ],
    );
    push_global_options(&mut out);
    out
}

fn complete_ls(current: &str, args_before: &[String]) -> Vec<Candidate> {
    if let Some((option, inline)) = value_option(args_before, current, &["--output"]) {
        return value_suggestions(&option, inline, &["json"]);
    }

    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("--json", "Shortcut for --output json"),
            ("--output", "Set output format"),
            ("--help", "Show help for ls"),
            ("-h", "Show help for ls"),
        ],
    );
    push_global_options(&mut out);
    out
}

fn complete_rm<P: WorkspaceProvider>(
    args_before: &[String],
    workspace_ctx: &mut WorkspaceContext<'_, P>,
) -> Vec<Candidate> {
    let mut has_all = false;
    let mut workspace_seen = false;

    for token in args_before {
        match token.as_str() {
            "--all" => has_all = true,
            _ if token.starts_with('-') => {}
            _ => {
                workspace_seen = true;
                break;
            }
        }
    }

    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("--all", "Remove all workspaces"),
            ("--yes", "Skip confirmation prompt"),
            ("-y", "Skip confirmation prompt"),
            ("--keep-volumes", "Keep attached volumes"),
            ("--volumes", "Remove attached volumes"),
            ("--help", "Show help for rm"),
            ("-h", "Show help for rm"),
        ],
    );
    push_global_options(&mut out);

    if !has_all && !workspace_seen {
        out.extend(workspace_ctx.workspace_candidates(None));
    }

    out
}

fn complete_exec<P: WorkspaceProvider>(
    current: &str,
    args_before: &[String],
    workspace_ctx: &mut WorkspaceContext<'_, P>,
) -> Vec<Candidate> {
    if let Some((option, inline)) = value_option(args_before, current, &["--user"]) {
        return value_suggestions(&option, inline, &[]);
    }

    let mut workspace_seen = false;
    let mut idx = 0usize;
    while idx < args_before.len() {
        let token = args_before[idx].as_str();
        match token {
            "--user" => idx += 2,
            _ if token.starts_with("--user=") => idx += 1,
            _ if token.starts_with('-') => idx += 1,
            _ => {
                workspace_seen = true;
                break;
            }
        }
    }

    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("--root", "Run command as root user"),
            ("--user", "Run command as specific user"),
            ("--help", "Show help for exec"),
            ("-h", "Show help for exec"),
        ],
    );
    push_global_options(&mut out);

    if !workspace_seen {
        out.extend(workspace_ctx.workspace_candidates(None));
    }

    out
}

fn complete_tunnel<P: WorkspaceProvider>(
    current: &str,
    args_before: &[String],
    workspace_ctx: &mut WorkspaceContext<'_, P>,
) -> Vec<Candidate> {
    if let Some((option, inline)) = value_option(args_before, current, &["--name", "--output"]) {
        return match option.as_str() {
            "--output" => value_suggestions(&option, inline, &["json"]),
            _ => value_suggestions(&option, inline, &[]),
        };
    }

    let mut workspace_seen = false;
    let mut idx = 0usize;
    while idx < args_before.len() {
        let token = args_before[idx].as_str();
        match token {
            "--name" | "--output" => idx += 2,
            _ if token.starts_with("--name=") || token.starts_with("--output=") => idx += 1,
            _ if token.starts_with('-') => idx += 1,
            _ => {
                workspace_seen = true;
                break;
            }
        }
    }

    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("--name", "Set tunnel display name"),
            ("--detach", "Start tunnel in background"),
            ("--output", "Set output format"),
            ("--help", "Show help for tunnel"),
            ("-h", "Show help for tunnel"),
        ],
    );
    push_global_options(&mut out);

    if !workspace_seen {
        out.extend(workspace_ctx.workspace_candidates(None));
    }

    out
}

fn complete_auth<P: WorkspaceProvider>(
    current: &str,
    args_before: &[String],
    workspace_ctx: &mut WorkspaceContext<'_, P>,
) -> Vec<Candidate> {
    if let Some((option, inline)) = value_option(
        args_before,
        current,
        &["--container", "--workspace", "--profile", "--host", "--key"],
    ) {
        return match option.as_str() {
            "--container" | "--workspace" => {
                workspace_ctx.workspace_candidates(Some((&option, inline)))
            }
            _ => value_suggestions(&option, inline, &[]),
        };
    }

    let mut provider: Option<String> = None;
    let mut workspace_seen = false;

    let mut idx = 0usize;
    while idx < args_before.len() {
        let token = args_before[idx].as_str();
        match token {
            "--container" | "--workspace" => {
                idx += 1;
                if idx < args_before.len() && !args_before[idx].is_empty() {
                    workspace_seen = true;
                }
            }
            "--profile" | "--host" | "--key" => {
                idx += 1;
            }
            _ if token.starts_with("--container=") || token.starts_with("--workspace=") => {
                workspace_seen = true;
            }
            _ if token.starts_with("--profile=")
                || token.starts_with("--host=")
                || token.starts_with("--key=") => {}
            _ if token.starts_with('-') => {}
            _ => {
                if provider.is_none() {
                    provider = Some(token.to_string());
                } else {
                    workspace_seen = true;
                }
            }
        }
        idx += 1;
    }

    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("--container", "Target workspace by container name"),
            ("--workspace", "Target workspace by workspace name"),
            ("--profile", "Select Codex profile"),
            ("--host", "Set GitHub host"),
            ("--key", "Set GPG key id"),
            ("--help", "Show help for auth"),
            ("-h", "Show help for auth"),
        ],
    );
    push_global_options(&mut out);

    if let Some(provider) = provider.as_deref() {
        match provider {
            "github" => push_described_values(
                &mut out,
                &[
                    ("--host", "Set GitHub host"),
                    ("--container", "Target workspace by container name"),
                    ("--workspace", "Target workspace by workspace name"),
                ],
            ),
            "codex" => push_described_values(
                &mut out,
                &[
                    ("--profile", "Select Codex profile"),
                    ("--container", "Target workspace by container name"),
                    ("--workspace", "Target workspace by workspace name"),
                ],
            ),
            "gpg" => push_described_values(
                &mut out,
                &[
                    ("--key", "Set GPG key id"),
                    ("--container", "Target workspace by container name"),
                    ("--workspace", "Target workspace by workspace name"),
                ],
            ),
            _ => {}
        }

        if !workspace_seen {
            out.extend(workspace_ctx.workspace_candidates(None));
        }
    } else {
        push_described_values(
            &mut out,
            &[
                ("github", "Sync GitHub token to workspace"),
                ("codex", "Sync Codex auth file to workspace"),
                ("gpg", "Import GPG signing key into workspace"),
            ],
        );
    }

    out
}

fn complete_reset<P: WorkspaceProvider>(
    current: &str,
    args_before: &[String],
    workspace_ctx: &mut WorkspaceContext<'_, P>,
) -> Vec<Candidate> {
    if args_before.is_empty() {
        let mut out: Vec<Candidate> = Vec::new();
        push_described_values(
            &mut out,
            &[
                ("repo", "Reset one repo to remote ref"),
                ("work-repos", "Reset all work repos"),
                ("opt-repos", "Reset repos under /opt"),
                ("private-repo", "Reset private repo"),
            ],
        );
        push_described_values(
            &mut out,
            &[
                ("--help", "Show help for reset"),
                ("-h", "Show help for reset"),
            ],
        );
        push_global_options(&mut out);
        return out;
    }

    let mut reset_subcommand: Option<(usize, &str)> = None;
    for (idx, token) in args_before.iter().enumerate() {
        if token.starts_with('-') {
            continue;
        }
        reset_subcommand = Some((idx, token.as_str()));
        break;
    }

    let Some((reset_idx, reset_cmd)) = reset_subcommand else {
        let mut out: Vec<Candidate> = Vec::new();
        push_described_values(
            &mut out,
            &[
                ("repo", "Reset one repo to remote ref"),
                ("work-repos", "Reset all work repos"),
                ("opt-repos", "Reset repos under /opt"),
                ("private-repo", "Reset private repo"),
            ],
        );
        push_described_values(
            &mut out,
            &[
                ("--help", "Show help for reset"),
                ("-h", "Show help for reset"),
            ],
        );
        push_global_options(&mut out);
        return out;
    };

    let nested_before = &args_before[(reset_idx + 1)..];
    match reset_cmd {
        "repo" => complete_reset_repo(current, nested_before, workspace_ctx),
        "work-repos" => complete_reset_work_repos(current, nested_before, workspace_ctx),
        "opt-repos" => complete_reset_simple(current, nested_before, workspace_ctx, false),
        "private-repo" => complete_reset_simple(current, nested_before, workspace_ctx, true),
        _ => {
            let mut out: Vec<Candidate> = Vec::new();
            push_described_values(
                &mut out,
                &[
                    ("repo", "Reset one repo to remote ref"),
                    ("work-repos", "Reset all work repos"),
                    ("opt-repos", "Reset repos under /opt"),
                    ("private-repo", "Reset private repo"),
                ],
            );
            out
        }
    }
}

fn complete_reset_repo<P: WorkspaceProvider>(
    current: &str,
    args_before: &[String],
    workspace_ctx: &mut WorkspaceContext<'_, P>,
) -> Vec<Candidate> {
    if let Some((option, inline)) = value_option(args_before, current, &["--ref"]) {
        return value_suggestions(&option, inline, &[]);
    }

    let workspace_seen = first_positional(args_before).is_some();

    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("--ref", "Set git ref to reset"),
            ("--yes", "Skip confirmation prompt"),
            ("-y", "Skip confirmation prompt"),
            ("--help", "Show help for reset repo"),
            ("-h", "Show help for reset repo"),
        ],
    );
    push_global_options(&mut out);

    if !workspace_seen {
        out.extend(workspace_ctx.workspace_candidates(None));
    }

    out
}

fn complete_reset_work_repos<P: WorkspaceProvider>(
    current: &str,
    args_before: &[String],
    workspace_ctx: &mut WorkspaceContext<'_, P>,
) -> Vec<Candidate> {
    if let Some((option, inline)) =
        value_option(args_before, current, &["--root", "--depth", "--ref"])
    {
        return value_suggestions(&option, inline, &[]);
    }

    let workspace_seen =
        first_positional_skipping_options(args_before, &["--root", "--depth", "--ref"]).is_some();

    let mut out: Vec<Candidate> = Vec::new();
    push_described_values(
        &mut out,
        &[
            ("--root", "Set repository root directory"),
            ("--depth", "Set fetch depth"),
            ("--ref", "Set git ref to reset"),
            ("--yes", "Skip confirmation prompt"),
            ("-y", "Skip confirmation prompt"),
            ("--help", "Show help for reset work-repos"),
            ("-h", "Show help for reset work-repos"),
        ],
    );
    push_global_options(&mut out);

    if !workspace_seen {
        out.extend(workspace_ctx.workspace_candidates(None));
    }

    out
}

fn complete_reset_simple<P: WorkspaceProvider>(
    current: &str,
    args_before: &[String],
    workspace_ctx: &mut WorkspaceContext<'_, P>,
    with_ref: bool,
) -> Vec<Candidate> {
    if with_ref && let Some((option, inline)) = value_option(args_before, current, &["--ref"]) {
        return value_suggestions(&option, inline, &[]);
    }

    let workspace_seen = first_positional(args_before).is_some();

    let mut out: Vec<Candidate> = Vec::new();
    if with_ref {
        push_described_values(
            &mut out,
            &[
                ("--ref", "Set git ref to reset"),
                ("--yes", "Skip confirmation prompt"),
                ("-y", "Skip confirmation prompt"),
                ("--help", "Show help for reset private-repo"),
                ("-h", "Show help for reset private-repo"),
            ],
        );
    } else {
        push_described_values(
            &mut out,
            &[
                ("--yes", "Skip confirmation prompt"),
                ("-y", "Skip confirmation prompt"),
                ("--help", "Show help for reset opt-repos"),
                ("-h", "Show help for reset opt-repos"),
            ],
        );
    }
    push_global_options(&mut out);

    if !workspace_seen {
        out.extend(workspace_ctx.workspace_candidates(None));
    }

    out
}

fn first_positional(args_before: &[String]) -> Option<&str> {
    for token in args_before {
        if token.starts_with('-') {
            continue;
        }
        return Some(token.as_str());
    }
    None
}

fn first_positional_skipping_options<'a>(
    args_before: &'a [String],
    options_with_values: &[&str],
) -> Option<&'a str> {
    let mut idx = 0usize;
    while idx < args_before.len() {
        let token = args_before[idx].as_str();

        if options_with_values.contains(&token) {
            idx += 2;
            continue;
        }

        let mut matched_inline = false;
        for option in options_with_values {
            let prefix = format!("{option}=");
            if token.starts_with(&prefix) {
                matched_inline = true;
                break;
            }
        }
        if matched_inline {
            idx += 1;
            continue;
        }

        if token.starts_with('-') {
            idx += 1;
            continue;
        }

        return Some(token);
    }

    None
}

fn complete_runtime_value(words_before: &[String], current: &str) -> Option<Vec<Candidate>> {
    if let Some(runtime_prefix) = current.strip_prefix("--runtime=") {
        return Some(with_prefix(
            "--runtime=",
            &filter_values(&["container", "host"], runtime_prefix),
        ));
    }

    if words_before.last().is_some_and(|last| last == "--runtime") {
        return Some(vec![
            Candidate::value("container"),
            Candidate::value("host"),
        ]);
    }

    None
}

fn detect_subcommand(words_before: &[String]) -> Option<(usize, &str)> {
    if words_before.len() <= 1 {
        return None;
    }

    let mut idx = 1usize;
    while idx < words_before.len() {
        let token = words_before[idx].as_str();
        if token == "--runtime" {
            idx += 2;
            continue;
        }

        if token.starts_with("--runtime=") {
            idx += 1;
            continue;
        }

        if token.starts_with('-') {
            idx += 1;
            continue;
        }

        return Some((idx, token));
    }

    None
}

fn strip_runtime_tokens(args_before: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(args_before.len());
    let mut idx = 0usize;

    while idx < args_before.len() {
        let token = args_before[idx].as_str();
        if token == "--runtime" {
            idx += 2;
            continue;
        }
        if token.starts_with("--runtime=") {
            idx += 1;
            continue;
        }

        out.push(args_before[idx].clone());
        idx += 1;
    }

    out
}

fn value_option(args_before: &[String], current: &str, options: &[&str]) -> Option<(String, bool)> {
    for option in options {
        let prefix = format!("{option}=");
        if current.starts_with(&prefix) {
            return Some(((*option).to_string(), true));
        }
    }

    if let Some(last) = args_before.last()
        && options.contains(&last.as_str())
    {
        return Some((last.clone(), false));
    }

    None
}

fn value_suggestions(option: &str, inline: bool, values: &[&str]) -> Vec<Candidate> {
    if inline {
        return with_prefix(&format!("{option}="), values);
    }

    let mut out: Vec<Candidate> = Vec::new();
    push_values(&mut out, values);
    out
}

fn filter_values<'a>(values: &'a [&'a str], prefix: &str) -> Vec<&'a str> {
    values
        .iter()
        .copied()
        .filter(|value| value.starts_with(prefix))
        .collect()
}

#[derive(Debug)]
struct WorkspaceContext<'a, P: WorkspaceProvider> {
    provider: &'a P,
    runtime: Runtime,
    cache: Option<Result<Vec<String>, String>>,
    workspace_error: Option<String>,
}

impl<'a, P: WorkspaceProvider> WorkspaceContext<'a, P> {
    fn new(provider: &'a P, runtime: Runtime) -> Self {
        Self {
            provider,
            runtime,
            cache: None,
            workspace_error: None,
        }
    }

    fn workspace_candidates(&mut self, option: Option<(&str, bool)>) -> Vec<Candidate> {
        let names = self.workspace_names();
        if names.is_empty() {
            return Vec::new();
        }

        let mut out: Vec<Candidate> = Vec::with_capacity(names.len());
        for name in names {
            let value = match option {
                Some((flag, true)) => format!("{flag}={name}"),
                _ => name,
            };
            out.push(Candidate::value(value));
        }
        out
    }

    fn workspace_names(&mut self) -> Vec<String> {
        if self.cache.is_none() {
            self.cache = Some(self.provider.list_workspaces(self.runtime));
        }

        match self.cache.as_ref() {
            Some(Ok(values)) => values.clone(),
            Some(Err(err)) => {
                if self.workspace_error.is_none() {
                    self.workspace_error = Some(err.clone());
                }
                Vec::new()
            }
            None => Vec::new(),
        }
    }
}
