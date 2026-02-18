use super::candidates::Candidate;
use super::protocol::{CompletionOutputFormat, CompletionShell};

pub(crate) fn render(
    shell: CompletionShell,
    format: CompletionOutputFormat,
    candidates: &[Candidate],
) -> String {
    match format {
        CompletionOutputFormat::Plain => render_plain(shell, candidates),
        CompletionOutputFormat::Describe => render_described(shell, candidates),
    }
}

fn render_plain(shell: CompletionShell, candidates: &[Candidate]) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        lines.push(escape_for_shell(shell, &candidate.value));
    }
    lines.join("\n")
}

fn render_described(shell: CompletionShell, candidates: &[Candidate]) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let value = escape_for_shell(shell, &candidate.value);
        match &candidate.description {
            Some(description) => {
                lines.push(format!("{value}\t{}", sanitize_segment(description)));
            }
            None => lines.push(value),
        }
    }
    lines.join("\n")
}

fn escape_for_shell(shell: CompletionShell, value: &str) -> String {
    let sanitized = sanitize_segment(value);
    match shell {
        CompletionShell::Bash | CompletionShell::Zsh => sanitized,
    }
}

fn sanitize_segment(value: &str) -> String {
    value.replace(['\n', '\r', '\t'], " ")
}
