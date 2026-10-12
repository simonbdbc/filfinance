//! Handler HTTP runtime HLS.

// `Json` et `State` exposent un snapshot runtime simple en JSON.
use axum::{Json, extract::State};
// `json!` evite un DTO dedie pour ce payload d'observabilite.
use serde_json::json;

// L'etat partage contient l'origin HLS et ses compteurs.
use crate::app_state::AppState;

/// Expose une vue synthetique des metriques runtime HLS.
pub async fn hls_runtime(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "hls": state.hls_origin.snapshot()
    }))
}
