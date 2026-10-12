//! Publication des evenements audio vers le control plane live.

use crate::{
    app_state::AppState,
    features::live::{
        dto::{
            EdgeAudioFailedPayload, EdgeAudioFinalizedPayload, EdgeAudioReadyPayload,
            EdgeLiveMessage, LiveNewsItem,
        },
        edge::{
            edge_event_id_for_audio_ready, publish_audio_failed, publish_audio_finalized,
            publish_audio_ready,
        },
    },
    shared::time::now_rfc3339,
};
use tracing::debug;

/// Publie `audio_ready` pour un clip HLS dont le manifest est déjà finalisé.
pub(in crate::features::audio::service::progressive) fn publish_finalized_audio_ready_control_plane(
    state: &AppState,
    live_item: &LiveNewsItem,
    stream_id: &str,
    playlist_url: &str,
) {
    let published_at = now_rfc3339();
    publish_audio_ready_control_plane(
        state,
        EdgeAudioReadyPayload {
            event_id: edge_event_id_for_audio_ready(live_item.id),
            server_time: published_at.clone(),
            translation_id: live_item.id,
            news_event_id: live_item.news_event_id,
            timestamp: live_item.timestamp.clone(),
            stream_id: stream_id.to_owned(),
            playlist_url: playlist_url.to_owned(),
            published_at,
            target_duration_secs: state.config.hls.clip_segment_target_duration_secs,
        },
    );
}

/// Publie `audio_ready` vers l'edge si configure et vers le fallback backend `/ws/news`.
fn publish_audio_ready_control_plane(state: &AppState, payload: EdgeAudioReadyPayload) {
    let event_id = payload.event_id.clone();
    state.publish_live_metadata(EdgeLiveMessage::AudioReady {
        payload: payload.clone(),
    });
    debug!(%event_id, "audio_ready metadata passed to the shared live publisher");
    publish_audio_ready(&state.config.edge, payload);
}

/// Publie `audio_failed` vers l'edge si configure et vers le fallback backend `/ws/news`.
pub(in crate::features::audio::service::progressive) fn publish_audio_failed_control_plane(
    state: &AppState,
    payload: EdgeAudioFailedPayload,
) {
    let event_id = payload.event_id.clone();
    state.publish_live_metadata(EdgeLiveMessage::AudioFailed {
        payload: payload.clone(),
    });
    debug!(%event_id, "audio_failed metadata passed to the shared live publisher");
    publish_audio_failed(&state.config.edge, payload);
}

/// Publie `audio_finalized` vers l'edge si configure et vers le fallback backend `/ws/news`.
pub(in crate::features::audio::service::progressive) fn publish_audio_finalized_control_plane(
    state: &AppState,
    payload: EdgeAudioFinalizedPayload,
) {
    let event_id = payload.event_id.clone();
    state.publish_live_metadata(EdgeLiveMessage::AudioFinalized {
        payload: payload.clone(),
    });
    debug!(%event_id, "audio_finalized metadata passed to the shared live publisher");
    publish_audio_finalized(&state.config.edge, payload);
}
