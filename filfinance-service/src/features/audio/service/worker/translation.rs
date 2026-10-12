//! Transformation d'une traduction prête en audio live.

mod admission;
mod body;
mod completion;
mod replay;
mod source;
mod trace;

use crate::features::audio::tts::provider::TtsProvider;

// `Instant` mesure le délai entre admission TTS et disponibilité HLS.
use std::time::Instant;

use crate::{
    app_state::AppState,
    features::{live::dto::LiveNewsItem, translation::domain::PersistedNewsTranslation},
};

use super::super::progressive::{publish_progressive_clip_failed, publish_progressive_clip_intro};
use admission::accept_live_audio_item;
use body::prepare_body_stream;
use completion::complete_prepared_stream;
use source::load_source_event;
use trace::{emit_tts_failed_trace, emit_tts_received_trace};

/// Traite une traduction prête pour la transformer en clip HLS audio.
pub(super) async fn handle_translation(
    state: &AppState,
    provider: &mut TtsProvider,
    translation: PersistedNewsTranslation,
) {
    // Ce repère mesure le délai entre l'admission TTS et la disponibilité HLS du stream.
    let tts_received_at = Instant::now();
    // Le `LiveNewsItem` reste la source de vérité pour les traces et les corrélations de diffusion.
    let source_event = load_source_event(state, translation.news_event_id).await;
    let live_item = LiveNewsItem::from_translation(
        translation.clone(),
        source_event.as_ref(),
        state.config.translation.max_body_chars,
    );
    let test_run_id = live_item.test_run_id.clone();

    emit_tts_received_trace(state, &live_item, test_run_id.as_deref());

    if !accept_live_audio_item(state, &live_item) {
        return;
    }

    let progressive_clip = publish_progressive_clip_intro(state, provider, &live_item).await;

    let prepared_stream = match prepare_body_stream(state, provider, &live_item).await {
        Ok(Some(prepared_stream)) => prepared_stream,
        Ok(None) => {
            state.tts_runtime.record_skipped();
            return;
        }
        Err(error) => {
            if let Some(clip) = progressive_clip.as_ref() {
                publish_progressive_clip_failed(
                    state,
                    Some(translation.id),
                    Some(clip.stream_id.clone()),
                    format!("body tts failed: {error}"),
                );
            }
            emit_tts_failed_trace(state, test_run_id.as_deref(), error.to_string());
            return;
        }
    };

    complete_prepared_stream(
        state,
        provider.provider_name(),
        &live_item,
        &prepared_stream,
        progressive_clip.as_ref(),
        test_run_id.as_deref(),
        tts_received_at,
    )
    .await;
}
