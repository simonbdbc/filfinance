//! Handler HTTP d'observabilite du runtime TTS.

// Json et State exposent un snapshot runtime simple en JSON.
use axum::{Json, extract::State};
// json evite un DTO dedie pour ce payload d'observabilite.
use serde_json::json;

use crate::app_state::AppState;

/// Expose une vue synthetique des metriques runtime TTS.
pub async fn tts_runtime(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "tts": state.tts_runtime.snapshot()
    }))
}
