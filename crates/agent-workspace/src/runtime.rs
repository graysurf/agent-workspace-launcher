use std::ffi::OsString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    Container,
    Host,
}

pub fn resolve_runtime(args: &[OsString]) -> Result<(Runtime, Vec<OsString>), String> {
    let mut runtime_from_flag: Option<Runtime> = None;
    let mut cleaned: Vec<OsString> = Vec::new();

    let mut idx = 0usize;
    while idx < args.len() {
        let current = args[idx].to_string_lossy();
        match current.as_ref() {
            "--runtime" => {
                idx += 1;
                if idx >= args.len() {
                    return Err(String::from("missing value for --runtime"));
                }
                runtime_from_flag =
                    Some(parse_runtime_value(args[idx].to_string_lossy().as_ref())?);
            }
            _ if current.starts_with("--runtime=") => {
                let value = &current["--runtime=".len()..];
                runtime_from_flag = Some(parse_runtime_value(value)?);
            }
            _ => cleaned.push(args[idx].clone()),
        }
        idx += 1;
    }

    let runtime = if let Some(runtime) = runtime_from_flag {
        runtime
    } else if let Ok(value) = std::env::var("AGENT_WORKSPACE_RUNTIME") {
        parse_runtime_value(&value)?
    } else if let Ok(value) = std::env::var("AWL_RUNTIME") {
        parse_runtime_value(&value)?
    } else {
        Runtime::Container
    };

    Ok((runtime, cleaned))
}

pub(crate) fn parse_runtime_value(raw: &str) -> Result<Runtime, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "container" | "docker" => Ok(Runtime::Container),
        "host" | "native" => Ok(Runtime::Host),
        "" => Err(String::from(
            "invalid runtime value: empty (expected: container|host)",
        )),
        _ => Err(format!(
            "invalid runtime value: {raw} (expected: container|host)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{Runtime, resolve_runtime};

    #[test]
    fn defaults_to_container() {
        let _guard = crate::env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        unsafe {
            std::env::remove_var("AGENT_WORKSPACE_RUNTIME");
            std::env::remove_var("AWL_RUNTIME");
        }
        let (runtime, cleaned) = resolve_runtime(&[]).expect("resolve runtime");
        assert_eq!(runtime, Runtime::Container);
        assert!(cleaned.is_empty());
    }

    #[test]
    fn flag_overrides_env() {
        let _guard = crate::env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        unsafe {
            std::env::set_var("AGENT_WORKSPACE_RUNTIME", "host");
        }
        let (runtime, cleaned) = resolve_runtime(&[
            "--runtime".into(),
            "container".into(),
            "create".into(),
            "--name".into(),
            "ws-demo".into(),
        ])
        .expect("resolve runtime");
        assert_eq!(runtime, Runtime::Container);
        assert_eq!(cleaned.len(), 3);

        unsafe {
            std::env::remove_var("AGENT_WORKSPACE_RUNTIME");
        }
    }

    #[test]
    fn parses_inline_flag_and_strips_it_from_args() {
        let (runtime, cleaned) = resolve_runtime(&[
            "--runtime=host".into(),
            "ls".into(),
            "--output".into(),
            "json".into(),
        ])
        .expect("resolve runtime");
        assert_eq!(runtime, Runtime::Host);
        assert_eq!(cleaned, vec!["ls", "--output", "json"]);
    }

    #[test]
    fn rejects_invalid_runtime() {
        let err = resolve_runtime(&["--runtime".into(), "k8s".into()]).expect_err("invalid");
        assert!(err.contains("expected: container|host"));
    }
}
