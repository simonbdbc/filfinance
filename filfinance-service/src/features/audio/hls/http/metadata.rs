//! Handler HTTP des metadonnees replay HLS.

// `Json` et `State` injectent l'etat applicatif dans le handler metadata.
use axum::{Json, extract::State};

// L'etat partage contient l'origin HLS.
use crate::app_state::AppState;

/// Retourne la metadata du replay HLS unique de la journee courante.
pub async fn get_replay_today(
    State(state): State<AppState>,
) -> Json<crate::features::audio::hls::HlsReplayTodayMetadata> {
    // Le client lit cette metadata protegee avant de lancer le player HLS public/cacheable.
    Json(state.hls_origin.replay_today_metadata())
}
