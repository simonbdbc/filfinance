//! Tests du contrat health public/admin.

use axum::{Json, extract::State};
use serde_json::{Value, json};

use super::http::{admin_health, health};
use crate::features::test_support::app_state;

#[tokio::test]
async fn public_health_is_minimal() {
    let Json(payload) = health().await;

    assert_eq!(payload, json!({ "status": "ok" }));
}

#[tokio::test]
async fn admin_health_redacts_sensitive_runtime_values() {
    let state = app_state().await;

    let Json(payload) = admin_health(State(state)).await;

    assert_eq!(payload["status"], Value::String("ok".to_owned()));
    assert!(payload.get("dependencies").is_some());
    assert_eq!(
        payload["dependencies"]["translation"]["worker_count"],
        Value::from(1)
    );
    assert_eq!(
        payload["dependencies"]["tts"]["worker_count"],
        Value::from(1)
    );
    assert!(
        payload["dependencies"]["pools"]["news_database"]["max_connections"]
            .as_u64()
            .expect("news pool max connections should be present")
            > 0
    );

    let serialized =
        serde_json::to_string(&payload).expect("admin health payload should serialize");
    assert!(!serialized.contains("test-browser-token"));
    assert!(!serialized.contains("data/hls-test"));
    assert!(!serialized.contains("ffmpeg_command"));
    assert!(!serialized.contains("access_ticket_secret"));
    assert!(!serialized.contains("NEWS_AUTH_JWKS_URL"));
}
