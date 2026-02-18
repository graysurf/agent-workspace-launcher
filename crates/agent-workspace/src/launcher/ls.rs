use std::ffi::OsString;

use crate::EXIT_RUNTIME;

use super::{PRIMARY_COMMAND_NAME, Workspace, json_escape, list_workspaces_on_disk};

#[derive(Debug, Default, Clone)]
pub(super) struct ParsedLs {
    pub(super) show_help: bool,
    pub(super) json: bool,
}

pub(super) fn parse_ls_args(args: &[OsString]) -> Result<ParsedLs, String> {
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

pub(super) fn run(args: &[OsString]) -> i32 {
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

fn print_ls_usage() {
    eprintln!("usage: {PRIMARY_COMMAND_NAME} ls [--json|--output json]");
}
