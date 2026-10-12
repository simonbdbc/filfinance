//! Synthese TTS d'un stream audio complet.

use std::time::Instant;

use tracing::error;

use super::super::super::tts::{
    normalization::normalize_narration_text_for_tts,
    provider::{TtsProvider, TtsRequest},
};
use super::{id::next_audio_stream_id, types::PreparedAudioStream};

use crate::features::audio::service::segmented_tts::synthesize_punctuation_segmented;
use crate::features::glossary::{domain::GlossaryListParams, pipeline::apply_spoken_glossary};
use crate::{app_state::AppState, errors::AppError, features::live::dto::LiveNewsItem};

/// Construit un stream audio complet a partir d'une traduction FR, sans choisir encore le transport.
pub(in crate::features::audio::service) async fn prepare_audio_stream(
    state: &AppState,
    provider: &mut TtsProvider,
    live_item: &LiveNewsItem,
    audio_text: &str,
) -> Result<Option<PreparedAudioStream>, AppError> {
    // Le texte audio est fourni par le caller pour distinguer intro, corps de clip et narration complete.
    let glossary_entries = match state
        .news
        .list_glossary_entries(&GlossaryListParams::active_for_pipeline())
        .await
    {
        Ok(entries) => entries,
        Err(error) => {
            state
                .translation_runtime
                .record_glossary_error(error.to_string());
            Vec::new()
        }
    };
    let (spoken_text, glossary_hits) = apply_spoken_glossary(audio_text, &glossary_entries);
    state
        .translation_runtime
        .record_glossary_hits(glossary_hits);
    let request = TtsRequest {
        text: normalize_narration_text_for_tts(&spoken_text),
    };

    if request.text.trim().is_empty() {
        return Ok(None);
    }

    let synthesis_started_at = Instant::now();
    let synthesized_audio = match synthesize_punctuation_segmented(provider, &request.text).await {
        Ok(audio) => audio,
        Err(error) => {
            let duration_ms = synthesis_started_at.elapsed().as_millis() as u64;
            state.tts_runtime.record_failed(format!(
                "synthesis failed after {}ms for translation {}: {}",
                duration_ms, live_item.id, error
            ));
            error!(
                %error,
                translation_id = live_item.id,
                news_event_id = live_item.news_event_id,
                duration_ms = duration_ms,
                "failed to synthesize audio stream"
            );
            return Err(error);
        }
    };

    let stream_id = next_audio_stream_id();
    let total_bytes = synthesized_audio.audio.bytes.len();
    let chunk_count = if total_bytes == 0 {
        0
    } else {
        synthesized_audio.segment_count
    };
    let synthesis_duration_ms = synthesis_started_at.elapsed().as_millis() as u64;
    let start_payload = crate::features::audio::domain::AudioStartPayload {
        stream_id: stream_id.clone(),
        translation_id: live_item.id,
        news_event_id: live_item.news_event_id,
        language: live_item.language.clone(),
        source: live_item.source.clone(),
        event_kind: live_item.event_kind.clone(),
        source_title: live_item.source_title.clone(),
        translated_title: live_item.translated_title.clone(),
        test_run_id: live_item.test_run_id.clone(),
        timestamp: live_item.timestamp.clone(),
        voice: synthesized_audio.audio.voice.clone(),
        audio_format: synthesized_audio.audio.audio_format.clone(),
        mime_type: synthesized_audio.audio.mime_type.clone(),
        total_bytes,
        chunk_count,
    };

    state.tts_runtime.record_generated(
        &stream_id,
        synthesis_duration_ms,
        total_bytes as u64,
        chunk_count,
    );

    Ok(Some(PreparedAudioStream {
        start_payload,
        audio_bytes: synthesized_audio.audio.bytes,
        synthesis_duration_ms,
    }))
}
