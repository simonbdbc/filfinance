//! Corps d'erreur reserves aux logs serveur, jamais aux contrats admin/SQLite.
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) const MAX_ERROR_LOG_BYTES: usize = 65_536;
const MAX_MESSAGE_BYTES: usize = 4_096;

// Ces patterns couvrent aussi les messages non JSON et les secrets dans une chaine JSON imbriquee.
static CREDENTIALS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:bearer\s+[a-z0-9._~+/=-]+|sk-or-v1-[a-z0-9_-]+|(?:api[_-]?key|access[_-]?token|refresh[_-]?token|password|secret)\s*[=:]\s*[^\s,;"}]+)"#)
        .expect("static error-log credential pattern")
});

/// Pas de Debug : seules les emissions explicites ci-dessous peuvent afficher le corps.
struct ErrorLog {
    code: Option<i64>,
    message: String,
    body: String,
    truncated: bool,
}

fn clip(value: &mut String, max: usize) -> bool {
    if value.len() <= max {
        return false;
    }
    let mut end = max;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    true
}

fn redact_text(value: &str, api_key: &str, prompt: &str) -> String {
    let mut value = value.to_owned();
    // Le secret reel est supprime avant toute troncature, meme s'il traverse la borne de log.
    for secret in [api_key, prompt] {
        if !secret.is_empty() {
            value = value.replace(secret, "[REDACTED]");
        }
    }
    CREDENTIALS.replace_all(&value, "[REDACTED]").into_owned()
}

fn redact_json(value: &mut Value, api_key: &str, prompt: &str) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                let key = key.to_ascii_lowercase().replace('-', "_");
                if matches!(
                    key.as_str(),
                    "authorization"
                        | "api_key"
                        | "apikey"
                        | "password"
                        | "secret"
                        | "token"
                        | "access_token"
                        | "refresh_token"
                        | "cookie"
                        | "set_cookie"
                        | "headers"
                        | "prompt"
                ) {
                    *value = Value::String("[REDACTED]".to_owned());
                } else {
                    redact_json(value, api_key, prompt);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_json(value, api_key, prompt);
            }
        }
        Value::String(value) => *value = redact_text(value, api_key, prompt),
        _ => {}
    }
}

fn numeric(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str()?.parse().ok())
}

impl ErrorLog {
    fn from_body(body: &[u8], api_key: &str, prompt: &str, incomplete: bool) -> Self {
        let parsed = serde_json::from_slice::<Value>(body).ok();
        let code = parsed.as_ref().and_then(|value| {
            [
                value.pointer("/error/code"),
                value.get("code"),
                value.get("statusCode"),
                value.get("status"),
            ]
            .into_iter()
            .flatten()
            .find_map(numeric)
        });
        let message = parsed
            .as_ref()
            .and_then(|value| {
                value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .or_else(|| value.get("error").and_then(Value::as_str))
                    .or_else(|| value.get("message").and_then(Value::as_str))
            })
            .unwrap_or("provider_message_unavailable");
        let mut message = redact_text(message, api_key, prompt);
        let mut logged_body = if let Some(mut value) = parsed {
            redact_json(&mut value, api_key, prompt);
            serde_json::to_string(&value).unwrap_or_else(|_| "body_serialization_failed".to_owned())
        } else {
            redact_text(&String::from_utf8_lossy(body), api_key, prompt)
        };
        let truncated_message = clip(&mut message, MAX_MESSAGE_BYTES);
        let truncated_body = clip(&mut logged_body, MAX_ERROR_LOG_BYTES);
        Self {
            code,
            message,
            body: logged_body,
            truncated: incomplete || truncated_message || truncated_body,
        }
    }
}

pub(super) fn emit(
    http_status: u16,
    model: &str,
    body: &[u8],
    api_key: &str,
    prompt: &str,
    incomplete: bool,
) {
    emit_inner(http_status, model, body, api_key, prompt, incomplete, true);
}

pub(super) fn emit_without_payload(
    http_status: u16,
    model: &str,
    body: &[u8],
    api_key: &str,
    prompt: &str,
    incomplete: bool,
) {
    emit_inner(http_status, model, body, api_key, prompt, incomplete, false);
}

fn emit_inner(
    http_status: u16,
    model: &str,
    body: &[u8],
    api_key: &str,
    prompt: &str,
    incomplete: bool,
    log_payload: bool,
) {
    if log_payload {
        crate::features::calendar_logs::event(
            "openrouter.response_body_status",
            serde_json::json!({
                "status": http_status,
                "captured_bytes": body.len(),
                "incomplete": incomplete,
            }),
        );
        crate::features::calendar_logs::payload(
            if incomplete {
                "openrouter.response_body_incomplete"
            } else {
                "openrouter.response_error_body"
            },
            &String::from_utf8_lossy(body).replace(api_key, "[REDACTED]"),
        );
    }
    let entry = ErrorLog::from_body(body, api_key, prompt, incomplete);
    // `?` echappe les retours ligne/caracteres de controle : le fournisseur ne peut forger une ligne de log.
    tracing::warn!(
        target: "calendar_openrouter_error",
        http_status, model, provider_code = ?entry.code,
        provider_message = ?entry.message, error_body = ?entry.body,
        truncated = entry.truncated, captured_bytes = body.len(),
        captured_sha256 = %format!("{:x}", Sha256::digest(body)),
        "calendar_openrouter_error_response"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn isolated_rlogger_keeps_bounded_error_body_beyond_stdout_excerpt() {
        let directory = tempfile::tempdir().expect("temporary log directory");
        let (logs, runtime) = crate::features::calendar_logs::CalendarLogs::start(
            directory.path().to_path_buf(),
            vec!["key-secret".into()],
            false,
        )
        .await;
        let body = format!("head key-secret {} tail-marker", "x".repeat(70_000));
        logs.scope(serde_json::json!({"run_id": "error-fixture"}), async {
            emit(
                500,
                "fixture",
                body.as_bytes(),
                "key-secret",
                "prompt",
                false,
            );
        })
        .await;
        runtime.shutdown().await;

        let mut written = String::new();
        let root = directory.path().join("rlogger");
        for day in std::fs::read_dir(root).expect("RLOGGER root") {
            let day = day.expect("day entry");
            if !day.file_type().expect("day type").is_dir()
                || day.file_name().to_string_lossy() == ".rlogger"
            {
                continue;
            }
            for instance in std::fs::read_dir(day.path()).expect("instance directory") {
                for file in std::fs::read_dir(instance.expect("instance").path()).expect("files") {
                    written.push_str(
                        &std::fs::read_to_string(file.expect("file").path()).expect("UTF-8 log"),
                    );
                }
            }
        }
        assert!(written.contains("tail-marker"));
        assert!(written.contains("openrouter.response_error_body"));
        assert!(!written.contains("key-secret"));
    }

    #[test]
    fn retains_numeric_code_message_and_body_without_credentials() {
        let body = br#"{"error":{"code":502,"message":"Upstream failed","metadata":{"provider":"fixture","raw":"provider details"}},"authorization":"Bearer hidden","api_key":"another-key"}"#;
        let entry = ErrorLog::from_body(body, "real-key", "real-prompt", false);
        assert_eq!(entry.code, Some(502));
        assert_eq!(entry.message, "Upstream failed");
        assert!(entry.body.contains("provider details"));
        assert!(!entry.body.contains("hidden"));
        assert!(!entry.body.contains("another-key"));
        assert!(!entry.truncated);
    }

    #[test]
    fn numeric_codes_and_invalid_bodies_remain_diagnosable_and_bounded() {
        for body in [
            r#"{"error":{"code":"429"}}"#,
            r#"{"statusCode":429}"#,
            r#"{"status":429}"#,
        ] {
            assert_eq!(
                ErrorLog::from_body(body.as_bytes(), "key", "prompt", false).code,
                Some(429)
            );
        }
        let raw = format!(
            "invalid JSON real-key Bearer abc.def\n{}",
            "é".repeat(MAX_ERROR_LOG_BYTES)
        );
        let entry = ErrorLog::from_body(raw.as_bytes(), "real-key", "prompt", false);
        assert!(entry.truncated);
        assert!(entry.body.len() <= MAX_ERROR_LOG_BYTES);
        assert!(!entry.body.contains("real-key"));
        assert!(!entry.body.contains("abc.def"));
        assert!(entry.body.starts_with("invalid JSON"));
    }
}
