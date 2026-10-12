//! Publication HLS replay du stream audio préparé.

// `Instant` mesure le délai entre admission TTS et disponibilité HLS.
use std::time::Instant;

// `warn` garde visibles les échecs HLS non bloquants.
use tracing::warn;

use crate::{app_state::AppState, features::live::dto::LiveNewsItem};

use super::super::super::stream::PreparedAudioStream;

/// Publie le stream complet dans le replay HLS du jour et trace le résultat si besoin.
pub(super) async fn publish_replay_stream(
    state: &AppState,
    live_item: &LiveNewsItem,
    prepared_stream: &PreparedAudioStream,
    stream_id: &str,
    test_run_id: Option<&str>,
    tts_received_at: Instant,
) {
    let metrics = match state
        .hls_origin
        .publish_replay_stream(&prepared_stream.start_payload, &prepared_stream.audio_bytes)
        .await
    {
        Ok(metrics) => metrics,
        Err(error) => {
            warn!(
                %error,
                translation_id = live_item.id,
                news_event_id = live_item.news_event_id,
                stream_id = %stream_id,
                "failed to publish tts stream to hls replay origin"
            );

            if let Some(run_id) = test_run_id {
                state.emit_test_trace(
                    run_id,
                    "hls_publish_failed",
                    "warning",
                    "Le stream audio HLS replay n'a pas pu etre publie.",
                    Some(error.to_string()),
                );
            }
            return;
        }
    };

    if !metrics.hls_enabled || metrics.segment_count == 0 {
        return;
    }

    let tts_to_playlist_ms = tts_received_at.elapsed().as_millis() as u64;
    if let Some(run_id) = test_run_id {
        state.emit_test_trace(
            run_id,
            "hls_published",
            "ok",
            "Le stream audio HLS est disponible dans le replay du jour.",
            Some(format!(
                "translation_id={} stream_id={} publish_duration_ms={} render_duration_ms={} segment_count={} tts_to_playlist_ms={}",
                live_item.id,
                stream_id,
                metrics.publish_duration_ms,
                metrics.render_duration_ms,
                metrics.segment_count,
                tts_to_playlist_ms
            )),
        );
    }
}
