//! Prechauffage du provider TTS.

// `Instant` mesure le cout du warmup expose dans le runtime.
use std::time::Instant;

// Les logs suivent les echecs de synthese sans arreter le service entier.
use tracing::{info, warn};

use crate::{app_state::AppState, features::audio::tts::provider::TtsProvider};

/// Réchauffe le provider TTS avant de consommer le flux live.
pub(super) async fn warmup_provider(state: &AppState, provider: &mut TtsProvider) {
    // Le warmup détecte tout de suite une voix Piper invalide ou un helper Python cassé.
    let warmup_started_at = Instant::now();
    match provider.warmup(&state.config.tts).await {
        Ok(()) => {
            let duration_ms = warmup_started_at.elapsed().as_millis() as u64;
            state.tts_runtime.record_warmup_success(duration_ms);
            info!(
                provider = provider.provider_name(),
                voice = %provider.voice(),
                warmup_duration_ms = duration_ms,
                "tts provider warmup completed"
            );
        }
        Err(error) => {
            let duration_ms = warmup_started_at.elapsed().as_millis() as u64;
            state.tts_runtime.record_failed(format!(
                "warmup failed after {}ms for {}: {}",
                duration_ms,
                provider.provider_name(),
                error
            ));
            warn!(
                %error,
                provider = provider.provider_name(),
                warmup_duration_ms = duration_ms,
                "tts provider warmup failed; the worker will continue and retry on the next news item"
            );
        }
    }
}
