use std::ffi::OsString;

use crate::runtime::{Runtime, parse_runtime_value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionShell {
    Bash,
    Zsh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionMode {
    Rust,
    Legacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionOutputFormat {
    Plain,
    Describe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionRequest {
    pub(crate) shell: CompletionShell,
    pub(crate) words: Vec<String>,
    pub(crate) cword: usize,
    pub(crate) runtime: Runtime,
    pub(crate) mode: CompletionMode,
    pub(crate) output: CompletionOutputFormat,
}

impl CompletionRequest {
    pub(crate) fn parse(args: &[OsString]) -> Result<Self, String> {
        let mut shell: Option<CompletionShell> = None;
        let mut cword: Option<usize> = None;
        let mut words_raw: Option<String> = None;
        let mut words: Vec<String> = Vec::new();
        let mut output = CompletionOutputFormat::Plain;

        let mut idx = 0usize;
        while idx < args.len() {
            let token = args[idx].to_string_lossy();
            match token.as_ref() {
                "--shell" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --shell"));
                    }
                    shell = Some(parse_shell(args[idx].to_string_lossy().as_ref())?);
                }
                "--cword" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --cword"));
                    }
                    cword = Some(parse_cword(args[idx].to_string_lossy().as_ref())?);
                }
                "--word" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --word"));
                    }
                    words.push(args[idx].to_string_lossy().into_owned());
                }
                "--words" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --words"));
                    }
                    words_raw = Some(args[idx].to_string_lossy().into_owned());
                }
                "--format" => {
                    idx += 1;
                    if idx >= args.len() {
                        return Err(String::from("missing value for --format"));
                    }
                    output = parse_output(args[idx].to_string_lossy().as_ref())?;
                }
                _ if token.starts_with("--shell=") => {
                    shell = Some(parse_shell(&token["--shell=".len()..])?);
                }
                _ if token.starts_with("--cword=") => {
                    cword = Some(parse_cword(&token["--cword=".len()..])?);
                }
                _ if token.starts_with("--word=") => {
                    words.push(token["--word=".len()..].to_string());
                }
                _ if token.starts_with("--words=") => {
                    words_raw = Some(token["--words=".len()..].to_string());
                }
                _ if token.starts_with("--format=") => {
                    output = parse_output(&token["--format=".len()..])?;
                }
                _ => {
                    return Err(format!("unknown option for __complete: {token}"));
                }
            }
            idx += 1;
        }

        if words.is_empty() {
            let raw = words_raw.ok_or_else(|| String::from("missing --word or --words"))?;
            words = parse_words_blob(&raw);
        }

        if words.is_empty() {
            return Err(String::from("completion words cannot be empty"));
        }

        let cword = cword.ok_or_else(|| String::from("missing --cword"))?;
        if cword >= words.len() {
            return Err(format!(
                "invalid --cword index {cword} for {} word(s)",
                words.len()
            ));
        }

        let shell = shell.ok_or_else(|| String::from("missing --shell"))?;
        let mode = parse_mode_from_env()?;
        let runtime = resolve_runtime_from_words(&words, cword);

        Ok(Self {
            shell,
            words,
            cword,
            runtime,
            mode,
            output,
        })
    }

    pub(crate) fn current_word(&self) -> &str {
        self.words
            .get(self.cword)
            .map(String::as_str)
            .unwrap_or_default()
    }

    pub(crate) fn words_before_cursor(&self) -> &[String] {
        &self.words[..self.cword]
    }
}

fn parse_shell(raw: &str) -> Result<CompletionShell, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bash" => Ok(CompletionShell::Bash),
        "zsh" => Ok(CompletionShell::Zsh),
        _ => Err(format!("invalid --shell value: {raw} (expected: bash|zsh)")),
    }
}

fn parse_output(raw: &str) -> Result<CompletionOutputFormat, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "plain" => Ok(CompletionOutputFormat::Plain),
        "describe" | "described" => Ok(CompletionOutputFormat::Describe),
        _ => Err(format!(
            "invalid --format value: {raw} (expected: plain|describe)"
        )),
    }
}

fn parse_cword(raw: &str) -> Result<usize, String> {
    raw.trim()
        .parse::<usize>()
        .map_err(|_| format!("invalid --cword value: {raw}"))
}

fn parse_words_blob(raw: &str) -> Vec<String> {
    let mut words: Vec<String> = raw.split_whitespace().map(ToString::to_string).collect();

    let has_trailing_whitespace = raw.chars().last().is_some_and(char::is_whitespace);
    if has_trailing_whitespace {
        words.push(String::new());
    }

    if words.is_empty() {
        words.push(String::new());
    }

    words
}

fn parse_mode_from_env() -> Result<CompletionMode, String> {
    let mode =
        std::env::var("AGENT_WORKSPACE_COMPLETION_MODE").unwrap_or_else(|_| String::from("rust"));
    parse_mode_value(&mode)
}

pub(crate) fn parse_mode_value(raw: &str) -> Result<CompletionMode, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "rust" => Ok(CompletionMode::Rust),
        "legacy" => Ok(CompletionMode::Legacy),
        _ => Err(format!(
            "invalid AGENT_WORKSPACE_COMPLETION_MODE value: {raw} (expected: rust|legacy)"
        )),
    }
}

fn resolve_runtime_from_words(words: &[String], cword: usize) -> Runtime {
    let mut runtime_from_flag: Option<Runtime> = None;
    let mut idx = 1usize;

    while idx < cword {
        let token = words[idx].trim();
        if token == "--runtime" {
            if idx + 1 < cword {
                if let Ok(runtime) = parse_runtime_value(words[idx + 1].trim()) {
                    runtime_from_flag = Some(runtime);
                }
                idx += 2;
                continue;
            }

            idx += 1;
            continue;
        }

        if let Some(value) = token.strip_prefix("--runtime=")
            && let Ok(runtime) = parse_runtime_value(value.trim())
        {
            runtime_from_flag = Some(runtime);
        }

        idx += 1;
    }

    if let Some(runtime) = runtime_from_flag {
        return runtime;
    }

    for key in ["AGENT_WORKSPACE_RUNTIME", "AWL_RUNTIME"] {
        if let Ok(value) = std::env::var(key)
            && let Ok(runtime) = parse_runtime_value(value.trim())
        {
            return runtime;
        }
    }

    Runtime::Container
}
