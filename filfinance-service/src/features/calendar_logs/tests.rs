use std::{fs, sync::Arc, time::Duration};

use serde_json::json;

use super::{CalendarLogs, DetailGate, event, observe, payload};

fn all_log_text(root: &std::path::Path) -> String {
    fn visit(path: &std::path::Path, output: &mut String) {
        for entry in fs::read_dir(path).expect("log directory should be readable") {
            let entry = entry.expect("directory entry should be readable");
            let entry_path = entry.path();
            if entry
                .file_type()
                .expect("file type should be readable")
                .is_dir()
            {
                visit(&entry_path, output);
            } else if entry_path
                .extension()
                .is_some_and(|extension| extension == "log")
            {
                output.push_str(&fs::read_to_string(entry_path).expect("log should be UTF-8"));
            }
        }
    }

    let mut output = String::new();
    visit(root, &mut output);
    output
}

#[tokio::test]
async fn detailed_payloads_are_fragmented_ordered_and_redacted() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let secret = "secret-token-42".to_owned();
    let (logs, runtime) =
        CalendarLogs::start(directory.path().to_path_buf(), vec![secret.clone()], false).await;
    let response_payload = format!("prompt complet: {secret}; réponse: {}", "é🦀".repeat(2_000));

    logs.scope(
        json!({ "local_day": "2026-09-10", "run_id": "run-1", "source_id": "fixture" }),
        async {
            event(
                "source.http_response",
                json!({ "status": 200, "headers": { "content-type": "text/plain" } }),
            );
            payload("source.response_body", "document source intégral");
            event("openrouter.http_request", json!({ "method": "POST" }));
            payload("openrouter.response_body", &response_payload);
            event(
                "tts.concatenated",
                json!({ "segment_count": 2, "bytes": 1024 }),
            );
        },
    )
    .await;
    runtime.shutdown().await;

    let written = all_log_text(directory.path());
    assert!(written.starts_with("RLOG/1"));
    assert!(written.contains("local_day=\"2026-09-10\""));
    assert!(written.contains("rlogger_run_id="));
    assert!(written.contains("action=\"openrouter.response_body\""));
    assert!(written.contains("action=\"source.http_response\""));
    assert!(written.contains("action=\"source.response_body\""));
    assert!(written.contains("action=\"openrouter.http_request\""));
    assert!(written.contains("action=\"tts.concatenated\""));
    assert!(written.contains("part=1"));
    assert!(written.contains("part=2"));
    assert!(!written.contains(&secret));
    assert!(written.contains("[REDACTED]"));
    assert!(written.contains("🦀"));
}

#[tokio::test]
async fn rlogger_storage_keeps_legacy_runs_untouched() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let old = root.join("run-old/calendar/2026-09-01");
    fs::create_dir_all(&old).expect("old day directory");
    fs::write(old.join("workflow.log"), "old").expect("old log");

    let (_logs, runtime) = CalendarLogs::start(root.to_path_buf(), vec![], false).await;
    runtime.shutdown().await;

    assert!(old.exists());
    assert!(ordered_local_logger::storage::recognized(
        &root.join("rlogger")
    ));
}

#[tokio::test]
async fn low_disk_suppresses_payload_but_preserves_progress() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let (mut logs, runtime) =
        CalendarLogs::start(directory.path().to_path_buf(), vec![], false).await;
    logs.context.detail_gate = Some(Arc::new(DetailGate::new(
        directory.path().join("rlogger"),
        u64::MAX,
    )));

    logs.scope(json!({"local_day": "2026-09-10"}), async {
        event("workflow.attempt.started", json!({"attempt": 1}));
        payload("openrouter.request", "sensitive full prompt");
        event("workflow.attempt.finished", json!({"outcome": "Retry"}));
    })
    .await;
    runtime.shutdown().await;

    let written = all_log_text(directory.path());
    assert!(written.contains("workflow.attempt.started"));
    assert!(written.contains("diagnostics.degraded"));
    assert!(written.contains("payload.suppressed"));
    assert!(written.contains("workflow.attempt.finished"));
    assert!(!written.contains("sensitive full prompt"));
}

#[tokio::test]
async fn progress_is_visible_in_active_segment_before_workflow_finishes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let (logs, runtime) = CalendarLogs::start(directory.path().to_path_buf(), vec![], false).await;

    logs.scope(json!({"run_id": "run-live"}), async {
        event("workflow.attempt.started", json!({"attempt": 1}));
        tracing::trace!(target: "html5ever::tokenizer", "parser trace must not be copied");
        let mut visible = false;
        for _ in 0..30 {
            if all_log_text(directory.path()).contains("workflow.attempt.started") {
                visible = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(visible, "the live RLOGGER segment did not flush promptly");
        let observed = observe("execute_ingestion", json!({}), async { "terminal_error" }).await;
        assert_eq!(observed, "terminal_error");
    })
    .await;
    runtime.shutdown().await;

    let written = all_log_text(directory.path());
    assert!(written.contains("execute_ingestion.finished"));
    assert!(!written.contains("execute_ingestion.succeeded"));
    assert!(!written.contains("parser trace must not be copied"));
}

#[tokio::test]
async fn brief_events_bound_debug_output_without_splitting_a_secret() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let secret = "secret-that-crosses-event-boundary";
    let (logs, runtime) =
        CalendarLogs::start(directory.path().to_path_buf(), vec![secret.into()], false).await;
    let verbose = format!("{}{} tail-marker", "x".repeat(2040), secret);
    logs.scope(json!({"run_id": "brief-event"}), async {
        event("workflow.progress", json!({"debug": verbose}));
    })
    .await;
    runtime.shutdown().await;

    let written = all_log_text(directory.path());
    assert!(written.contains("brief_event_truncated"));
    assert!(!written.contains(secret));
    assert!(!written.contains("tail-marker"));
}
