//! Finalisation d'un stream audio prepare.

// `Instant` garde le repère d'admission TTS pour les métriques replay.
use std::time::Instant;

// Les logs suivent les streams générés sans arrêter le service entier.
use tracing::info;

use crate::{
    app_state::AppState,
    features::{
        audio::service::{progressive::ProgressiveClipContext, stream::PreparedAudioStream},
        live::dto::LiveNewsItem,
    },
};

use super::{replay::publish_replay_stream, trace::emit_tts_body_generated_trace};
use crate::features::audio::service::progressive::finalize_progressive_clip_body;

/// Finalise les sorties HLS, publie le replay et trace le stream genere.
pub(super) async fn complete_prepared_stream(
    state: &AppState,
    provider_name: &str,
    live_item: &LiveNewsItem,
    prepared_stream: &PreparedAudioStream,
    progressive_clip: Option<&ProgressiveClipContext>,
    test_run_id: Option<&str>,
    tts_received_at: Instant,
) {
    // Les payloads préparés restent réutilisables pour le replay et les clips HLS.
    let stream_id = prepared_stream.start_payload.stream_id.clone();
    let total_bytes = prepared_stream.start_payload.total_bytes;
    let chunk_count = prepared_stream.start_payload.chunk_count;
    let synthesis_duration_ms = prepared_stream.synthesis_duration_ms;

    if let Some(clip) = progressive_clip {
        finalize_progressive_clip_body(state, clip, &prepared_stream.audio_bytes, live_item).await;
    }

    publish_replay_stream(
        state,
        live_item,
        prepared_stream,
        &stream_id,
        test_run_id,
        tts_received_at,
    )
    .await;

    info!(
        provider = provider_name,
        voice = %prepared_stream.start_payload.voice,
        translation_id = live_item.id,
        news_event_id = live_item.news_event_id,
        stream_id = %stream_id,
        total_bytes = total_bytes,
        chunk_count = chunk_count,
        "tts audio stream generated for hls clip"
    );

    emit_tts_body_generated_trace(
        state,
        live_item,
        &stream_id,
        total_bytes,
        synthesis_duration_ms,
        test_run_id,
    );
}
