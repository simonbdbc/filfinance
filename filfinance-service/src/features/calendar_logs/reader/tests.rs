use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::any,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;
use tower::ServiceExt;

use super::{
    endpoint,
    files::FileRoot,
    management::{self, Hub},
    protocol::ContentOptions,
};

fn managed_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().expect("temporary directory");
    let root = temp.path().join("rlogger");
    ordered_local_logger::storage::initialize(&root).expect("initialize rlogger storage");
    let root = fs::canonicalize(root).expect("canonical rlogger root");
    ordered_local_logger::storage::open_file(
        &ordered_local_logger::storage::lease_path(&root, "1-2-3"),
        true,
        true,
    )
    .expect("run lease");
    let path = PathBuf::from("1970-01-01/calendar/workflow-00-1-2-3-h0-s1.log");
    fs::create_dir_all(root.join(path.parent().expect("parent"))).expect("log parent");
    fs::write(root.join(&path), "RLOG/1\nfinished\n").expect("finished log");
    (temp, root, path)
}

async fn inventory(root: &Path) -> management::Inventory {
    let hub = Hub::default();
    let job = hub.get(root.to_path_buf());
    job.kick(true);
    tokio::time::timeout(Duration::from_secs(5), async {
        while job.snapshot().busy {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("inventory finishes");
    job.snapshot()
}

fn file_id(root: &mut FileRoot, name: &str) -> String {
    root.entries(&root.node_id.clone(), None)
        .expect("root entries")
        .entries
        .into_iter()
        .find(|entry| entry.name == name)
        .expect("file entry")
        .id
}

#[test]
fn reader_preserves_unicode_append_and_detects_replacement() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("workflow.log");
    fs::write(&path, "RLOG/1 début 🦀\n").expect("initial content");
    let mut root = FileRoot::create(directory.path().to_str().unwrap()).expect("open root");
    let id = file_id(&mut root, "workflow.log");
    let first = root
        .content(&id, ContentOptions::default())
        .expect("first chunk");
    fs::write(&path, "RLOG/1 début 🦀\nsuite é\n").expect("appended content");
    let appended = root
        .content(
            &id,
            ContentOptions {
                after: Some(first.end.clone()),
                generation: Some(first.generation.clone()),
                ..ContentOptions::default()
            },
        )
        .expect("append chunk");
    let mut received = STANDARD.decode(first.data).expect("base64 first chunk");
    received.extend(
        STANDARD
            .decode(appended.data)
            .expect("base64 appended chunk"),
    );
    assert_eq!(
        String::from_utf8(received).unwrap(),
        "RLOG/1 début 🦀\nsuite é\n"
    );

    fs::write(&path, "nouveau fichier\n").expect("replacement");
    let error = root
        .content(
            &id,
            ContentOptions {
                after: Some(appended.end),
                generation: Some(first.generation),
                ..ContentOptions::default()
            },
        )
        .expect_err("replacement must reset the generation");
    assert!(matches!(
        error.code.as_ref(),
        "GENERATION_CHANGED" | "CURSOR_INVALID"
    ));
}

#[test]
fn finalized_segments_require_their_current_identity_for_management() {
    let (_temp, root, path) = managed_fixture();
    let identity = ordered_local_logger::storage::token(
        &fs::metadata(root.join(&path)).expect("finished metadata"),
    );

    assert!(management::file_state(&root, &path, 3600).1);
    assert!(!management::file_state(&root, &path, 3599).1);
    let download = management::action_file(&root, &path, &identity, false)
        .expect("finalized segment opens for download");
    assert_eq!(
        management::action_file(&root, &path, &identity, true)
            .expect_err("locked download prevents deletion")
            .code,
        "FILE_BUSY"
    );
    drop(download);
    drop(
        management::action_file(&root, &path, &identity, true)
            .expect("unlocked finalized segment allows deletion"),
    );
    fs::write(root.join(&path), "replacement").expect("replace fixture");
    assert_eq!(
        management::action_file(&root, &path, &identity, true)
            .expect_err("stale identity is refused")
            .code,
        "FILE_CHANGED"
    );
}

#[tokio::test]
async fn inventory_recovers_only_an_abandoned_active_segment_and_preserves_its_bytes() {
    let (_temp, root, path) = managed_fixture();
    let active = path.with_file_name("workflow-00-1-2-3-h0-s1.active.log");
    fs::rename(root.join(&path), root.join(&active)).expect("activate fixture");
    let content = "partial 🦀 without newline";
    fs::write(root.join(&active), content).expect("partial log");

    let lease = ordered_local_logger::storage::open_file(
        &ordered_local_logger::storage::lease_path(&root, "1-2-3"),
        true,
        false,
    )
    .expect("lease");
    lease.lock().expect("hold live lease");
    inventory(&root).await;
    assert!(root.join(&active).exists());
    drop(lease);

    inventory(&root).await;
    let recovered = active.with_file_name("workflow-00-1-2-3-h0-s1.recovered.log");
    assert_eq!(fs::read_to_string(root.join(&recovered)).unwrap(), content);
    assert!(!root.join(&active).exists());
}

#[tokio::test]
async fn http_reader_uses_the_configured_directory_and_requires_an_admin_context() {
    let directory = tempfile::tempdir().expect("temporary directory");
    fs::create_dir_all(directory.path().join("run/calendar/2026-09-10")).expect("logger directory");
    fs::write(
        directory
            .path()
            .join("run/calendar/2026-09-10/workflow.log"),
        "RLOG/1 sample\n",
    )
    .expect("logger file");

    let mut state = crate::features::test_support::app_state().await;
    state.calendar_logs = super::super::CalendarLogs::test_reader(directory.path().to_path_buf());
    let app = Router::new()
        .route("/api/admin/calendar-day/logs/{*path}", any(endpoint))
        .with_state(state);
    let admin = crate::features::auth::CurrentUser {
        id: "admin-test".to_owned(),
        email: None,
        role: "admin".to_owned(),
        access_token_expires_at: None,
    };

    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/calendar-day/logs/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("reader response");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let (_, session) = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/admin/calendar-day/logs/session")
                    .extension(admin.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("session response"),
    )
    .await;
    let token = session["token"].as_str().expect("session token");

    let (_, root) = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/admin/calendar-day/logs/roots")
                    .header("x-local-session", token)
                    .extension(admin.clone())
                    .body(Body::from(r#"{"absolutePath":"/must-not-be-used"}"#))
                    .unwrap(),
            )
            .await
            .expect("root response"),
    )
    .await;
    let expected_root = fs::canonicalize(directory.path()).expect("canonical temporary root");
    assert_eq!(
        root["absolutePath"],
        expected_root.to_string_lossy().as_ref()
    );

    let (status, _) = response_json(
        app.oneshot(
            Request::builder()
                .uri("/api/admin/calendar-day/logs/roots/unrelated/entries")
                .header("x-local-session", token)
                .extension(admin)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("confined reader response"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

async fn response_json(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("response body");
    (status, serde_json::from_slice(&bytes).expect("JSON body"))
}
