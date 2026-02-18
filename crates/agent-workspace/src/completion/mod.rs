mod candidates;
mod engine;
mod output;
mod protocol;
mod providers;

use std::ffi::OsString;

use engine::complete;
use protocol::{CompletionMode, CompletionRequest};
use providers::workspaces::RuntimeWorkspaceProvider;

use crate::EXIT_RUNTIME;

pub(crate) fn run(args: &[OsString]) -> i32 {
    let request = match CompletionRequest::parse(args) {
        Ok(request) => request,
        Err(err) => {
            eprintln!("error: {err}");
            return EXIT_RUNTIME;
        }
    };

    if request.mode == CompletionMode::Legacy {
        return 0;
    }

    let provider = RuntimeWorkspaceProvider;
    let result = complete(&request, &provider);
    let rendered = output::render(request.shell, request.output, &result.candidates);

    if !rendered.is_empty() {
        println!("{rendered}");
    }

    if let Some(err) = result.workspace_error {
        eprintln!("warn: completion workspace lookup failed: {err}");
    }

    0
}

#[cfg(test)]
mod protocol_tests {
    use std::ffi::OsString;

    use crate::runtime::Runtime;

    use super::protocol::{
        CompletionMode, CompletionOutputFormat, CompletionRequest, CompletionShell,
    };

    fn parse(args: &[&str]) -> Result<CompletionRequest, String> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        CompletionRequest::parse(&args)
    }

    #[test]
    fn parses_repeated_word_arguments() {
        let request = parse(&[
            "--shell",
            "bash",
            "--cword",
            "1",
            "--word",
            "agent-workspace-launcher",
            "--word",
            "",
        ])
        .expect("parse request");

        assert_eq!(request.shell, CompletionShell::Bash);
        assert_eq!(request.cword, 1);
        assert_eq!(request.words, vec!["agent-workspace-launcher", ""]);
        assert_eq!(request.runtime, Runtime::Container);
        assert_eq!(request.mode, CompletionMode::Rust);
        assert_eq!(request.output, CompletionOutputFormat::Plain);
    }

    #[test]
    fn parses_words_blob_with_trailing_space() {
        let request = parse(&[
            "--shell",
            "zsh",
            "--cword",
            "1",
            "--words",
            "agent-workspace-launcher ",
        ])
        .expect("parse words blob");

        assert_eq!(request.shell, CompletionShell::Zsh);
        assert_eq!(request.words, vec!["agent-workspace-launcher", ""]);
    }

    #[test]
    fn rejects_invalid_shell() {
        let err = parse(&[
            "--shell",
            "fish",
            "--cword",
            "1",
            "--word",
            "agent-workspace-launcher",
            "--word",
            "",
        ])
        .expect_err("invalid shell should fail");
        assert!(err.contains("expected: bash|zsh"));
    }

    #[test]
    fn rejects_out_of_range_cword() {
        let err = parse(&[
            "--shell",
            "bash",
            "--cword",
            "2",
            "--word",
            "agent-workspace-launcher",
            "--word",
            "",
        ])
        .expect_err("cword range check");
        assert!(err.contains("invalid --cword index"));
    }

    #[test]
    fn resolves_runtime_from_completed_tokens_before_cursor() {
        let request = parse(&[
            "--shell",
            "bash",
            "--cword",
            "4",
            "--word",
            "agent-workspace-launcher",
            "--word",
            "--runtime",
            "--word",
            "host",
            "--word",
            "rm",
            "--word",
            "",
        ])
        .expect("parse request with runtime");

        assert_eq!(request.runtime, Runtime::Host);
    }
}

#[cfg(test)]
mod mode_tests {
    use super::protocol::{CompletionMode, parse_mode_value};

    #[test]
    fn default_mode_is_rust() {
        assert_eq!(
            parse_mode_value("").expect("parse default mode"),
            CompletionMode::Rust
        );
        assert_eq!(
            parse_mode_value("rust").expect("parse explicit rust mode"),
            CompletionMode::Rust
        );
    }

    #[test]
    fn parses_legacy_mode_from_env() {
        assert_eq!(
            parse_mode_value("legacy").expect("parse legacy mode"),
            CompletionMode::Legacy
        );
    }

    #[test]
    fn rejects_unknown_mode_value() {
        let err = parse_mode_value("invalid").expect_err("invalid mode should fail");
        assert!(err.contains("rust|legacy"));
    }
}

#[cfg(test)]
mod engine_tests {
    use crate::runtime::Runtime;

    use super::engine::{WorkspaceProvider, complete};
    use super::output::render;
    use super::protocol::{CompletionOutputFormat, CompletionRequest, CompletionShell};

    #[derive(Debug, Clone)]
    struct StubWorkspaceProvider {
        host: Vec<String>,
        container: Vec<String>,
    }

    impl WorkspaceProvider for StubWorkspaceProvider {
        fn list_workspaces(&self, runtime: Runtime) -> Result<Vec<String>, String> {
            Ok(match runtime {
                Runtime::Host => self.host.clone(),
                Runtime::Container => self.container.clone(),
            })
        }
    }

    fn request(words: &[&str], cword: usize) -> CompletionRequest {
        let mut args: Vec<std::ffi::OsString> = vec![
            "--shell".into(),
            "bash".into(),
            "--cword".into(),
            cword.to_string().into(),
        ];

        for word in words {
            args.push("--word".into());
            args.push((*word).into());
        }

        CompletionRequest::parse(&args).expect("parse completion request")
    }

    fn provider() -> StubWorkspaceProvider {
        StubWorkspaceProvider {
            host: vec!["host-ws".to_string()],
            container: vec!["container-ws".to_string()],
        }
    }

    #[test]
    fn top_level_completion_returns_core_subcommands() {
        let result = complete(&request(&["agent-workspace-launcher", ""], 1), &provider());
        let values: Vec<String> = result
            .candidates
            .into_iter()
            .map(|candidate| candidate.value)
            .collect();

        for expected in ["auth", "create", "ls", "rm", "exec", "reset", "tunnel"] {
            assert!(values.iter().any(|value| value == expected));
        }
    }

    #[test]
    fn describe_output_contains_subcommand_descriptions() {
        let result = complete(&request(&["agent-workspace-launcher", ""], 1), &provider());
        let rendered = render(
            CompletionShell::Zsh,
            CompletionOutputFormat::Describe,
            &result.candidates,
        );

        assert!(rendered.contains("auth\tUpdate auth material in workspace"));
        assert!(rendered.contains("create\tCreate a new workspace"));
        assert!(rendered.contains("--runtime\tSelect runtime backend (container or host)"));
    }

    #[test]
    fn runtime_value_completion_is_available() {
        let result = complete(
            &request(&["agent-workspace-launcher", "--runtime", ""], 2),
            &provider(),
        );
        let values: Vec<String> = result
            .candidates
            .into_iter()
            .map(|candidate| candidate.value)
            .collect();

        assert_eq!(values, vec!["container", "host"]);
    }

    #[test]
    fn rm_workspace_candidates_follow_runtime() {
        let host_request = request(
            &["agent-workspace-launcher", "--runtime", "host", "rm", ""],
            4,
        );
        let host_result = complete(&host_request, &provider());
        let host_values: Vec<String> = host_result
            .candidates
            .into_iter()
            .map(|candidate| candidate.value)
            .collect();
        assert!(host_values.iter().any(|value| value == "host-ws"));
        assert!(!host_values.iter().any(|value| value == "container-ws"));

        let container_request = request(
            &[
                "agent-workspace-launcher",
                "--runtime",
                "container",
                "rm",
                "",
            ],
            4,
        );
        let container_result = complete(&container_request, &provider());
        let container_values: Vec<String> = container_result
            .candidates
            .into_iter()
            .map(|candidate| candidate.value)
            .collect();
        assert!(container_values.iter().any(|value| value == "container-ws"));
        assert!(!container_values.iter().any(|value| value == "host-ws"));
    }

    #[test]
    fn auth_workspace_completion_after_provider() {
        let result = complete(
            &request(
                &[
                    "agent-workspace-launcher",
                    "--runtime",
                    "host",
                    "auth",
                    "github",
                    "",
                ],
                5,
            ),
            &provider(),
        );
        let values: Vec<String> = result
            .candidates
            .into_iter()
            .map(|candidate| candidate.value)
            .collect();

        assert!(values.iter().any(|value| value == "host-ws"));
        assert!(values.iter().any(|value| value == "--host"));
    }

    #[test]
    fn reset_completion_lists_nested_subcommands() {
        let result = complete(
            &request(&["agent-workspace-launcher", "reset", ""], 2),
            &provider(),
        );
        let values: Vec<String> = result
            .candidates
            .into_iter()
            .map(|candidate| candidate.value)
            .collect();

        for expected in ["repo", "work-repos", "opt-repos", "private-repo"] {
            assert!(values.iter().any(|value| value == expected));
        }
    }
}

#[cfg(test)]
mod workspace_provider_tests {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    use crate::runtime::Runtime;

    use super::engine::WorkspaceProvider;
    use super::providers::workspaces::RuntimeWorkspaceProvider;

    #[test]
    fn host_workspace_provider_reads_workspace_root() {
        let _guard = crate::env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("host-a")).expect("create workspace a");
        fs::create_dir_all(temp.path().join("host-b")).expect("create workspace b");

        unsafe {
            std::env::set_var("AGENT_WORKSPACE_HOME", temp.path());
        }

        let provider = RuntimeWorkspaceProvider;
        let workspaces = provider
            .list_workspaces(Runtime::Host)
            .expect("list host workspaces");
        assert_eq!(workspaces, vec!["host-a", "host-b"]);

        unsafe {
            std::env::remove_var("AGENT_WORKSPACE_HOME");
        }
    }

    #[test]
    fn container_workspace_provider_uses_docker_list_output() {
        let _guard = crate::env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let temp = tempfile::tempdir().expect("tempdir");
        let bin_dir = temp.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("create bin dir");

        let docker_path = bin_dir.join("docker");
        let mut file = fs::File::create(&docker_path).expect("create docker stub");
        writeln!(
            file,
            "#!/usr/bin/env bash\nset -euo pipefail\nif [[ \"${{1:-}}\" == \"ps\" ]]; then\n  printf '%s\\n' 'container-a' 'container-b'\n  exit 0\nfi\nexit 1"
        )
        .expect("write docker stub");
        fs::set_permissions(&docker_path, fs::Permissions::from_mode(0o755))
            .expect("chmod docker stub");

        let existing_path = std::env::var("PATH").unwrap_or_default();
        let merged_path = format!("{}:{}", bin_dir.display(), existing_path);
        unsafe {
            std::env::set_var("PATH", merged_path);
        }

        let provider = RuntimeWorkspaceProvider;
        let workspaces = provider
            .list_workspaces(Runtime::Container)
            .expect("list container workspaces");
        assert_eq!(workspaces, vec!["container-a", "container-b"]);
    }
}

#[cfg(test)]
mod output_tests {
    use super::candidates::Candidate;
    use super::output::render;
    use super::protocol::{CompletionOutputFormat, CompletionShell};

    #[test]
    fn plain_output_sanitizes_control_characters() {
        let rendered = render(
            CompletionShell::Bash,
            CompletionOutputFormat::Plain,
            &[Candidate::value("--runtime=host\nnext")],
        );
        assert_eq!(rendered, "--runtime=host next");
    }

    #[test]
    fn described_output_includes_description_columns() {
        let rendered = render(
            CompletionShell::Zsh,
            CompletionOutputFormat::Describe,
            &[Candidate::described("rm", "Remove workspace")],
        );
        assert_eq!(rendered, "rm\tRemove workspace");
    }
}

#[cfg(test)]
mod degradation_tests {
    use crate::runtime::Runtime;

    use super::engine::{WorkspaceProvider, complete};
    use super::protocol::CompletionRequest;

    struct FailingWorkspaceProvider;

    impl WorkspaceProvider for FailingWorkspaceProvider {
        fn list_workspaces(&self, _runtime: Runtime) -> Result<Vec<String>, String> {
            Err(String::from("backend unavailable"))
        }
    }

    fn request(words: &[&str], cword: usize) -> CompletionRequest {
        let mut args: Vec<std::ffi::OsString> = vec![
            "--shell".into(),
            "bash".into(),
            "--cword".into(),
            cword.to_string().into(),
        ];

        for word in words {
            args.push("--word".into());
            args.push((*word).into());
        }

        CompletionRequest::parse(&args).expect("parse completion request")
    }

    #[test]
    fn workspace_lookup_failure_keeps_static_candidates() {
        let result = complete(
            &request(&["agent-workspace-launcher", "rm", ""], 2),
            &FailingWorkspaceProvider,
        );

        let values: Vec<String> = result
            .candidates
            .iter()
            .map(|candidate| candidate.value.clone())
            .collect();
        assert!(values.iter().any(|value| value == "--all"));
        assert!(result.workspace_error.is_some());
    }
}

#[cfg(test)]
mod matrix_tests {
    use super::engine::{WorkspaceProvider, complete};
    use super::protocol::CompletionRequest;
    use crate::runtime::Runtime;

    #[derive(Debug, Clone)]
    struct StubWorkspaceProvider;

    impl WorkspaceProvider for StubWorkspaceProvider {
        fn list_workspaces(&self, runtime: Runtime) -> Result<Vec<String>, String> {
            Ok(match runtime {
                Runtime::Host => vec!["host-ws".to_string()],
                Runtime::Container => vec!["container-ws".to_string()],
            })
        }
    }

    #[test]
    fn matrix_cases_cover_completion_surface() {
        let fixtures = include_str!("fixtures/matrix_cases.tsv");
        for line in fixtures.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            assert_eq!(parts.len(), 4, "invalid fixture line: {line}");

            let name = parts[0].trim();
            let words: Vec<&str> = parts[1].split(';').collect();
            let cword = parts[2]
                .trim()
                .parse::<usize>()
                .expect("fixture cword should parse");
            let expected: Vec<&str> = parts[3].split(';').collect();

            let mut args: Vec<std::ffi::OsString> = vec![
                "--shell".into(),
                "bash".into(),
                "--cword".into(),
                cword.to_string().into(),
            ];
            for word in &words {
                args.push("--word".into());
                args.push((*word).into());
            }

            let request = CompletionRequest::parse(&args).expect("parse fixture request");
            let result = complete(&request, &StubWorkspaceProvider);
            let values: Vec<String> = result
                .candidates
                .iter()
                .map(|candidate| candidate.value.clone())
                .collect();

            for expected_value in expected {
                assert!(
                    values.iter().any(|value| value == expected_value),
                    "case {name}: expected candidate {expected_value} missing; got {values:?}"
                );
            }
        }
    }
}
