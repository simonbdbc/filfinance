//! Handlers HTTP des resumes calendrier HLS persistants.

use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    app_state::AppState,
    features::audio::hls::access::{append_hls_ticket_to_playlist, verify_hls_access_ticket},
};

use super::headers::{clip_playlist_response_headers, segment_response_headers};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct HlsAccessQuery {
    hls_ticket: Option<String>,
}

/// Handler HTTP du pointeur stable `current.m3u8` pour un jour calendrier.
pub async fn calendar_summary_current_playlist(
    State(state): State<AppState>,
    AxumPath(local_day): AxumPath<String>,
    Query(query): Query<HlsAccessQuery>,
) -> Response {
    if let Err(error) = verify_hls_access_ticket(&state.config.hls, query.hls_ticket.as_deref()) {
        return error.into_response();
    }

    match state
        .hls_origin
        .calendar_summary_playlist(&local_day, None)
        .await
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
            (clip_playlist_response_headers(true), body).into_response()
        }
        Err(error) => error.into_response(),
    }
}

/// Handler HTTP du manifest immutable d'une generation calendrier.
pub async fn calendar_summary_immutable_playlist(
    State(state): State<AppState>,
    AxumPath((local_day, stream_id)): AxumPath<(String, String)>,
    Query(query): Query<HlsAccessQuery>,
) -> Response {
    if let Err(error) = verify_hls_access_ticket(&state.config.hls, query.hls_ticket.as_deref()) {
        return error.into_response();
    }

    match state
        .hls_origin
        .calendar_summary_playlist(&local_day, Some(&stream_id))
        .await
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
            (clip_playlist_response_headers(true), body).into_response()
        }
        Err(error) => error.into_response(),
    }
}

/// Handler HTTP d'un segment immutable de resume calendrier.
pub async fn calendar_summary_segment(
    State(state): State<AppState>,
    AxumPath((local_day, stream_id, segment_name)): AxumPath<(String, String, String)>,
    Query(query): Query<HlsAccessQuery>,
) -> Response {
    if let Err(error) = verify_hls_access_ticket(&state.config.hls, query.hls_ticket.as_deref()) {
        return error.into_response();
    }

    match state
        .hls_origin
        .calendar_summary_segment_bytes(&local_day, &stream_id, &segment_name)
        .await
    {
        Ok(bytes) => (StatusCode::OK, segment_response_headers(), bytes).into_response(),
        Err(error) => error.into_response(),
    }
}
