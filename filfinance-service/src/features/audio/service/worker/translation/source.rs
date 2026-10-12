//! Chargement de l'événement source pour enrichir le flux audio.

// `warn` trace les échecs d'enrichissement sans bloquer la TTS.
use tracing::warn;

use crate::{app_state::AppState, features::news::domain::PersistedNewsEvent};

/// Charge l'événement source quand il est encore disponible en base pour enrichir le flux audio.
pub(super) async fn load_source_event(
    state: &AppState,
    news_event_id: i64,
) -> Option<PersistedNewsEvent> {
    match state.news.get_news_by_id(news_event_id).await {
        Ok(event) => event,
        Err(error) => {
            warn!(%error, news_event_id, "failed to load source event for tts stream");
            None
        }
    }
}
