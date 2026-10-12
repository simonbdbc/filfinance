//! Préparation du corps audio TTS pour un item live.

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        audio::{
            service::stream::{
                PreparedAudioStream, build_live_clip_body_text, prepare_audio_stream,
            },
            tts::provider::TtsProvider,
        },
        live::dto::LiveNewsItem,
    },
};

/// Prépare le stream audio complet qui finalisera le clip progressif et alimentera le replay.
pub(super) async fn prepare_body_stream(
    state: &AppState,
    provider: &mut TtsProvider,
    live_item: &LiveNewsItem,
) -> Result<Option<PreparedAudioStream>, AppError> {
    // Le texte de corps est reconstruit une seule fois pour garder le même wording entre TTS et traces.
    let clip_body_text = build_live_clip_body_text(live_item);
    prepare_audio_stream(state, provider, live_item, &clip_body_text).await
}
