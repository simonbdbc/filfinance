//! Handlers HTTP du replay HLS du jour.

// `Path` et `State` injectent les parametres URL et l'etat applicatif.
// `IntoResponse` garde les handlers lisibles sans type de retour enum dedie.
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

// L'etat partage contient l'origin HLS.
use crate::app_state::AppState;
use crate::features::audio::hls::access::{
    append_hls_ticket_to_playlist, verify_hls_access_ticket,
};

use super::headers::{playlist_response_headers, segment_response_headers};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct HlsAccessQuery {
    hls_ticket: Option<String>,
    start_translation_id: Option<i64>,
}

/// Handler HTTP de la playlist replay HLS du jour.
pub async fn replay_today_playlist(
    State(state): State<AppState>,
    Query(query): Query<HlsAccessQuery>,
) -> Response {
    state.hls_origin.record_replay_playlist_request();
    if let Err(error) = verify_hls_access_ticket(&state.config.hls, query.hls_ticket.as_deref()) {
        return error.into_response();
    }

    match state
        .hls_origin
        .replay_today_playlist(query.start_translation_id)
    {
        Ok(playlist) => {
            let body = if state.config.hls.access_control_enabled {
                append_hls_ticket_to_playlist(
                    &playlist,
                    query.hls_ticket.as_deref().unwrap_or_default(),
                )
            } else {
                playlist
            };
            (playlist_response_headers(), body).into_response()
        }
        Err(error) => error.into_response(),
    }
}

/// Handler HTTP d'un segment replay HLS du jour.
pub async fn replay_today_segment(
    State(state): State<AppState>,
    AxumPath((local_day, segment_name)): AxumPath<(String, String)>,
    Query(query): Query<HlsAccessQuery>,
) -> Response {
    state.hls_origin.record_replay_segment_request();
    if let Err(error) = verify_hls_access_ticket(&state.config.hls, query.hls_ticket.as_deref()) {
        return error.into_response();
    }

    match state
        .hls_origin
        .replay_today_segment_bytes(&local_day, &segment_name)
        .await
    {
        Ok(bytes) => (StatusCode::OK, segment_response_headers(), bytes).into_response(),
        Err(error) => error.into_response(),
    }
}
