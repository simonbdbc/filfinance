//! Publication des echecs audio progressifs.

use crate::{
    app_state::AppState,
    features::live::{dto::EdgeAudioFailedPayload, edge::edge_event_id_for_audio_failed},
    shared::time::now_rfc3339,
};

use super::control_plane::publish_audio_failed_control_plane;

/// Publie un echec audio edge sans bloquer le pipeline TTS principal.
pub(in crate::features::audio::service) fn publish_progressive_clip_failed(
    state: &AppState,
    translation_id: Option<i64>,
    stream_id: Option<String>,
    message: String,
) {
    let failed_at = now_rfc3339();
    publish_audio_failed_control_plane(
        state,
        EdgeAudioFailedPayload {
            event_id: edge_event_id_for_audio_failed(translation_id),
            server_time: failed_at.clone(),
            translation_id,
            stream_id,
            error_message: message,
            failed_at,
        },
    );
}
