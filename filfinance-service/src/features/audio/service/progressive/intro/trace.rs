//! Logs de publication de l'intro HLS.

use std::time::Instant;

use tracing::info;

use crate::features::{audio::hls::origin::HlsClipInitialPublish, live::dto::LiveNewsItem};

use super::super::timing::elapsed_since_rfc3339_ms;

/// Journalise la publication d'intro avant finalisation du clip.
pub(super) fn trace_intro_published(
    live_item: &LiveNewsItem,
    initial_publish: &HlsClipInitialPublish,
    clip_started_at: Instant,
    synthesis_started_at: Instant,
) {
    let news_received_to_audio_ready_ms = elapsed_since_rfc3339_ms(&live_item.timestamp);
    let audio_ready_pipeline_duration_ms = clip_started_at.elapsed().as_millis() as u64;
    info!(
        translation_id = live_item.id,
        news_event_id = live_item.news_event_id,
        stream_id = %initial_publish.stream_id,
        intro_synthesis_duration_ms = synthesis_started_at.elapsed().as_millis() as u64,
        publish_duration_ms = initial_publish.publish_duration_ms,
        render_duration_ms = initial_publish.render_duration_ms,
        segment_count = initial_publish.segment_count,
        intro_pipeline_duration_ms = audio_ready_pipeline_duration_ms,
        news_received_to_audio_ready_ms = news_received_to_audio_ready_ms.unwrap_or(0),
        news_received_to_audio_ready_ms_available = news_received_to_audio_ready_ms.is_some(),
        "progressive hls clip intro staged"
    );
}
