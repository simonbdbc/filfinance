//! Warmup du provider de traduction.

use std::time::Instant;

use tracing::{info, warn};

use crate::{app_state::AppState, features::translation::provider::TranslationProvider};

/// Prechauffe le provider pour reveler vite une configuration cassee.
pub(super) async fn warmup_provider(state: &AppState, provider: &mut TranslationProvider) {
    // Le warmup charge a froid le provider et revele vite une config cassee.
    let warmup_started_at = Instant::now();
    match provider.warmup(&state.config.translation).await {
        Ok(()) => {
            let duration_ms = warmup_started_at.elapsed().as_millis() as u64;
            state.translation_runtime.record_warmup_success(duration_ms);
            info!(
                provider = provider.provider_name(),
                warmup_duration_ms = duration_ms,
                "translation provider warmup completed"
            );
        }
        Err(error) => {
            let duration_ms = warmup_started_at.elapsed().as_millis() as u64;
            state.translation_runtime.record_error(format!(
                "warmup failed after {}ms for {}: {}",
                duration_ms,
                provider.provider_name(),
                error
            ));
            warn!(
                %error,
                provider = provider.provider_name(),
                warmup_duration_ms = duration_ms,
                "translation provider warmup failed; the worker will continue with runtime fallback"
            );
        }
    }
}
