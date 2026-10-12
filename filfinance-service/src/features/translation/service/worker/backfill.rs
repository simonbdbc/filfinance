//! Backfill de demarrage du worker de traduction.

use tracing::{info, warn};

use crate::{app_state::AppState, features::translation::provider::TranslationProvider};

use super::super::backfill::backfill_recent_missing_translations;

/// Rattrape au demarrage les traductions recentes manquantes.
pub(super) async fn run_startup_backfill(state: &AppState, provider: &mut TranslationProvider) {
    match backfill_recent_missing_translations(state, provider).await {
        Ok(backfilled_count) => {
            if backfilled_count > 0 {
                info!(
                    provider = provider.provider_name(),
                    backfilled_count, "translation startup backfill completed"
                );
            }
        }
        Err(error) => {
            warn!(
                %error,
                provider = provider.provider_name(),
                "translation startup backfill failed"
            );
        }
    }
}
