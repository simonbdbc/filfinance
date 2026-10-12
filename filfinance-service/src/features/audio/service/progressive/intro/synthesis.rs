//! Synthese TTS de l'intro d'un clip HLS progressif.

use tracing::warn;

use crate::{
    app_state::AppState,
    features::{
        audio::{
            service::segmented_tts::synthesize_punctuation_segmented,
            tts::{
                normalization::normalize_narration_text_for_tts,
                provider::{SynthesizedAudio, TtsProvider},
            },
        },
        live::dto::LiveNewsItem,
        news::domain::build_news_narration_prefix_from_timestamp,
    },
};

use super::super::failure::publish_progressive_clip_failed;

/// Synthetise uniquement le prefixe audible du clip progressif.
pub(super) async fn synthesize_intro_audio(
    state: &AppState,
    provider: &mut TtsProvider,
    live_item: &LiveNewsItem,
    stream_id: &str,
) -> Option<SynthesizedAudio> {
    // L'intro audio reprend exactement le prefixe narration expose dans le texte user-facing.
    let intro_text = format!(
        "{}.",
        build_news_narration_prefix_from_timestamp(
            &live_item.timestamp,
            live_item.news_level.as_deref(),
        )
    );
    let normalized_text = normalize_narration_text_for_tts(&intro_text);

    match synthesize_punctuation_segmented(provider, &normalized_text).await {
        Ok(audio) => Some(audio.audio),
        Err(error) => {
            publish_progressive_clip_failed(
                state,
                Some(live_item.id),
                Some(stream_id.to_owned()),
                format!("intro tts failed: {error}"),
            );
            warn!(
                %error,
                translation_id = live_item.id,
                news_event_id = live_item.news_event_id,
                "failed to synthesize progressive hls clip intro"
            );
            None
        }
    }
}
