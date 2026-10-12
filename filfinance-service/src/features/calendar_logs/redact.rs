//! Redact credentials, not prompts or model responses. Runs before fragmentation.
use regex::Regex;
use std::sync::LazyLock;

static SECRET_FIELDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r#"(?i)(["']?(?:authorization|proxy-authorization|api[_-]?key|access[_-]?token|refresh[_-]?token|password|secret|cookie|set-cookie)["']?\s*[:=]\s*)(?:"(?:\\.|[^"\\])*"|'[^']*'|[^\s,;}]+)"#
).expect("credential fields regex")
});
static BEARER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:bearer\s+[a-z0-9._~+/=-]+|sk-or-v1-[a-z0-9_-]+|https?://[^/@\s]+:[^/@\s]+@)",
    )
    .expect("credential text regex")
});

pub(super) fn text(value: &str, secrets: &[String]) -> String {
    let mut value = value.to_owned();
    for secret in secrets.iter().filter(|s| !s.is_empty()) {
        value = value.replace(secret, "[REDACTED]");
        // Also cover credentials echoed inside JSON string bodies.
        if let Ok(encoded) = serde_json::to_string(secret) {
            value = value.replace(&encoded[1..encoded.len() - 1], "[REDACTED]");
        }
    }
    let value = SECRET_FIELDS.replace_all(&value, "$1\"[REDACTED]\"");
    BEARER.replace_all(&value, "[REDACTED]").into_owned()
}

pub(crate) fn environment_secrets() -> Vec<String> {
    std::env::vars()
        .filter_map(|(key, value)| {
            let key = key.to_ascii_uppercase();
            (key.contains("TOKEN")
                || key.contains("SECRET")
                || key.contains("PASSWORD")
                || key.ends_with("_KEY"))
            .then_some(value)
            .filter(|value| !value.is_empty())
        })
        .collect()
}
