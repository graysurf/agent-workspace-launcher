use std::ffi::OsString;

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};

pub const PRIMARY_BIN_NAME: &str = "agent-workspace-launcher";
pub const AWL_ALIAS_NAME: &str = "awl";
pub const CLI_VERSION: &str = match option_env!("AWL_RELEASE_VERSION") {
    Some(version) if !version.is_empty() => version,
    _ => env!("CARGO_PKG_VERSION"),
};

#[derive(Debug, Parser)]
#[command(
    name = "agent-workspace-launcher",
    version,
    about = "Workspace lifecycle CLI (container + host runtimes)",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "container|host",
        help = "Select runtime backend (default: container)"
    )]
    pub runtime: Option<String>,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    #[command(disable_help_flag = true)]
    Auth(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Create(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Ls(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Rm(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Exec(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Reset(PassthroughArgs),
    #[command(disable_help_flag = true)]
    Tunnel(PassthroughArgs),
}

#[derive(Debug, Args)]
#[command(trailing_var_arg = true)]
pub struct PassthroughArgs {
    #[arg(
        value_name = "ARG",
        num_args = 0..,
        allow_hyphen_values = true,
        value_parser = clap::builder::OsStringValueParser::new()
    )]
    pub args: Vec<OsString>,
}

pub struct ForwardRequest {
    pub subcommand: &'static str,
    pub args: Vec<OsString>,
}

impl CliCommand {
    pub fn into_forward_request(self) -> ForwardRequest {
        match self {
            Self::Auth(args) => ForwardRequest {
                subcommand: "auth",
                args: args.args,
            },
            Self::Create(args) => ForwardRequest {
                subcommand: "create",
                args: args.args,
            },
            Self::Ls(args) => ForwardRequest {
                subcommand: "ls",
                args: args.args,
            },
            Self::Rm(args) => ForwardRequest {
                subcommand: "rm",
                args: args.args,
            },
            Self::Exec(args) => ForwardRequest {
                subcommand: "exec",
                args: args.args,
            },
            Self::Reset(args) => ForwardRequest {
                subcommand: "reset",
                args: args.args,
            },
            Self::Tunnel(args) => ForwardRequest {
                subcommand: "tunnel",
                args: args.args,
            },
        }
    }
}

impl Cli {
    fn command_with_invocation_name(invocation_name: Option<&str>) -> clap::Command {
        let mut command = Self::command().version(CLI_VERSION);
        if let Some(name) = invocation_name.filter(|name| !name.is_empty()) {
            command = if name == AWL_ALIAS_NAME {
                command.name(AWL_ALIAS_NAME)
            } else {
                command.name(PRIMARY_BIN_NAME)
            };
        }
        command
    }

    pub fn try_parse_from_with_invocation<I, T>(
        args: I,
        invocation_name: Option<&str>,
    ) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let args_vec: Vec<OsString> = args.into_iter().map(Into::into).collect();

        let mut command = Self::command_with_invocation_name(invocation_name);
        let matches = command.clone().try_get_matches_from(args_vec)?;
        Self::from_arg_matches(&matches).map_err(|err| err.format(&mut command))
    }

    pub fn into_forward_request(self) -> ForwardRequest {
        let mut request = self.command.into_forward_request();
        if let Some(runtime) = self.runtime {
            request
                .args
                .insert(0, OsString::from(format!("--runtime={runtime}")));
        }
        request
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{AWL_ALIAS_NAME, CLI_VERSION, Cli, CliCommand, PRIMARY_BIN_NAME};

    #[test]
    fn help_lists_core_subcommands() {
        let mut cmd = Cli::command();
        let mut out = Vec::new();
        cmd.write_long_help(&mut out).expect("write help");

        let help = String::from_utf8(out).expect("utf8");
        for subcommand in ["auth", "create", "ls", "rm", "exec", "reset", "tunnel"] {
            assert!(
                help.contains(subcommand),
                "help should include subcommand {subcommand}"
            );
        }
    }

    #[test]
    fn parse_reset_keeps_trailing_args() {
        let cli = Cli::try_parse_from([
            PRIMARY_BIN_NAME,
            "reset",
            "repo",
            "ws-test",
            "--yes",
            "--ref",
            "origin/main",
        ])
        .expect("parse args");

        let CliCommand::Reset(args) = cli.command else {
            panic!("expected reset command");
        };
        let collected: Vec<String> = args
            .args
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            collected,
            vec!["repo", "ws-test", "--yes", "--ref", "origin/main"]
        );
    }

    #[test]
    fn parse_from_awl_alias_uses_same_command_tree() {
        let cli = Cli::try_parse_from_with_invocation(
            [AWL_ALIAS_NAME, "ls", "--output", "json"],
            Some(AWL_ALIAS_NAME),
        )
        .expect("parse alias args");

        let CliCommand::Ls(args) = cli.command else {
            panic!("expected ls command");
        };
        let collected: Vec<String> = args
            .args
            .into_iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(collected, vec!["--output", "json"]);
    }

    #[test]
    fn command_version_uses_release_override_when_available() {
        let cmd = Cli::command_with_invocation_name(None);
        assert_eq!(cmd.get_version(), Some(CLI_VERSION));
    }

    #[test]
    fn parse_runtime_global_after_subcommand() {
        let cli = Cli::try_parse_from([
            PRIMARY_BIN_NAME,
            "ls",
            "--runtime",
            "host",
            "--output",
            "json",
        ])
        .expect("parse runtime after subcommand");

        assert_eq!(cli.runtime.as_deref(), Some("host"));
        let request = cli.into_forward_request();
        let collected: Vec<String> = request
            .args
            .iter()
            .map(|item| item.to_string_lossy().into_owned())
            .collect();
        assert_eq!(collected, vec!["--runtime=host", "--output", "json"]);
    }
}
