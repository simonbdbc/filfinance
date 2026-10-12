//! Handlers HTTP des clips HLS progressifs.

// `Path` et `State` injectent les parametres URL et l'etat applicatif.
// `IntoResponse` garde les handlers lisibles sans type de retour enum dedie.
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
// `info` trace les hits origin utiles pour diagnostiquer le cache CDN.
use tracing::info;

// L'etat partage contient l'origin HLS.
use crate::app_state::AppState;

use super::headers::{clip_playlist_response_headers, segment_response_headers};
use crate::features::audio::hls::access::{
    append_hls_ticket_to_playlist, verify_hls_access_ticket,
};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct HlsAccessQuery {
    hls_ticket: Option<String>,
}

/// Handler HTTP de la playlist d'un clip HLS progressif.
pub async fn clip_playlist(
    State(state): State<AppState>,
    AxumPath(stream_id): AxumPath<String>,
    Query(query): Query<HlsAccessQuery>,
) -> Response {
    state.hls_origin.record_clip_playlist_request();
    if let Err(error) = verify_hls_access_ticket(&state.config.hls, query.hls_ticket.as_deref()) {
        return error.into_response();
    }

    match state.hls_origin.clip_playlist(&stream_id) {
        Ok(playlist) => {
            // Chaque hit playlist est journalise pour compter les rechargements par clip.
            info!(
                stream_id = %stream_id,
                finalized = playlist.finalized,
                "hls clip playlist requested"
            );
            let body = if state.config.hls.access_control_enabled {
                append_hls_ticket_to_playlist(
                    &playlist.body,
                    query.hls_ticket.as_deref().unwrap_or_default(),
                )
            } else {
                playlist.body
            };
            (clip_playlist_response_headers(playlist.finalized), body).into_response()
        }
        Err(error) => error.into_response(),
    }
}

/// Handler HTTP d'un segment de clip HLS progressif.
pub async fn clip_segment(
    State(state): State<AppState>,
    AxumPath((stream_id, segment_name)): AxumPath<(String, String)>,
    Query(query): Query<HlsAccessQuery>,
) -> Response {
    state.hls_origin.record_clip_segment_request();
    if let Err(error) = verify_hls_access_ticket(&state.config.hls, query.hls_ticket.as_deref()) {
        return error.into_response();
    }

    match state
        .hls_origin
        .clip_segment_bytes(&stream_id, &segment_name)
        .await
    {
        Ok(bytes) => {
            // Les segments sont immuables; ce log sert de compteur CDN/origin par segment.
            info!(
                stream_id = %stream_id,
                segment_name = %segment_name,
                "hls clip segment requested"
            );
            (StatusCode::OK, segment_response_headers(), bytes).into_response()
        }
        Err(error) => error.into_response(),
    }
}
