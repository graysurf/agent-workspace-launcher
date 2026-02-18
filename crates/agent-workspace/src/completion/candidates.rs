use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub(crate) value: String,
    pub(crate) description: Option<String>,
}

impl Candidate {
    pub(crate) fn value(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            description: None,
        }
    }

    pub(crate) fn described(value: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            description: Some(description.into()),
        }
    }
}

pub(crate) const GLOBAL_OPTIONS: &[(&str, &str)] = &[
    ("--runtime", "Select runtime backend (container or host)"),
    ("--help", "Show help for command"),
    ("--version", "Show CLI version"),
    ("-h", "Show help for command"),
    ("-V", "Show CLI version"),
];

pub(crate) fn push_values(target: &mut Vec<Candidate>, values: &[&str]) {
    for value in values {
        target.push(Candidate::value(*value));
    }
}

pub(crate) fn push_described_values(target: &mut Vec<Candidate>, values: &[(&str, &str)]) {
    for (value, description) in values {
        target.push(Candidate::described(*value, *description));
    }
}

pub(crate) fn push_global_options(target: &mut Vec<Candidate>) {
    push_described_values(target, GLOBAL_OPTIONS);
}

pub(crate) fn with_prefix(prefix: &str, values: &[&str]) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::with_capacity(values.len());
    for value in values {
        out.push(Candidate::value(format!("{prefix}{value}")));
    }
    out
}

pub(crate) fn finalize(candidates: Vec<Candidate>, prefix: &str) -> Vec<Candidate> {
    let mut deduped: BTreeMap<String, Option<String>> = BTreeMap::new();

    for candidate in candidates {
        if !candidate.value.starts_with(prefix) {
            continue;
        }
        let entry = deduped.entry(candidate.value).or_insert(None);
        if entry.is_none() && candidate.description.is_some() {
            *entry = candidate.description;
        }
    }

    deduped
        .into_iter()
        .map(|(value, description)| Candidate { value, description })
        .collect()
}
