//! Construction des payloads de demarrage audio HLS.

use super::super::super::{domain::AudioStartPayload, tts::provider::SynthesizedAudio};

use crate::features::live::dto::LiveNewsItem;

/// Construit le payload minimal necessaire a l'origin clip a partir de l'item live deja normalise.
pub(in crate::features::audio::service) fn audio_start_payload_for_clip(
    live_item: &LiveNewsItem,
    stream_id: &str,
    audio: &SynthesizedAudio,
    total_bytes: usize,
) -> AudioStartPayload {
    let chunk_count = if total_bytes == 0 { 0 } else { 1 };
    AudioStartPayload {
        stream_id: stream_id.to_owned(),
        translation_id: live_item.id,
        news_event_id: live_item.news_event_id,
        language: live_item.language.clone(),
        source: live_item.source.clone(),
        event_kind: live_item.event_kind.clone(),
        source_title: live_item.source_title.clone(),
        translated_title: live_item.translated_title.clone(),
        test_run_id: live_item.test_run_id.clone(),
        timestamp: live_item.timestamp.clone(),
        voice: audio.voice.clone(),
        audio_format: audio.audio_format.clone(),
        mime_type: audio.mime_type.clone(),
        total_bytes,
        chunk_count,
    }
}
