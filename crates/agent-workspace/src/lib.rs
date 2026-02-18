mod cli;
mod completion;
mod launcher;
mod runtime;

use std::path::Path;

use cli::{AWL_ALIAS_NAME, Cli, PRIMARY_BIN_NAME};

pub const EXIT_RUNTIME: i32 = 1;

pub fn run() -> i32 {
    run_with_args(std::env::args_os())
}

pub fn run_with_args<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args_vec: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    let invocation_name = detect_invocation_name(args_vec.first());

    let cli = match Cli::try_parse_from_with_invocation(args_vec, invocation_name.as_deref()) {
        Ok(parsed) => parsed,
        Err(err) => {
            let code = err.exit_code();
            let _ = err.print();
            return code;
        }
    };

    let request = cli.into_forward_request();
    if request.subcommand == "__complete" {
        return completion::run(&request.args);
    }
    launcher::dispatch(request.subcommand, &request.args)
}

fn detect_invocation_name(argv0: Option<&std::ffi::OsString>) -> Option<String> {
    let argv0 = argv0?;
    let basename = Path::new(argv0).file_name()?.to_string_lossy();
    if basename == AWL_ALIAS_NAME {
        return Some(AWL_ALIAS_NAME.to_string());
    }
    Some(PRIMARY_BIN_NAME.to_string())
}

#[cfg(test)]
pub(crate) fn env_lock() -> &'static std::sync::Mutex<()> {
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::run_with_args;

    #[test]
    fn run_with_args_accepts_awl_alias_help() {
        let exit_code = run_with_args(["awl", "--help"]);
        assert_eq!(exit_code, 0);
    }
}
