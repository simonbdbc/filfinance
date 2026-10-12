//! Construction des payloads WebSocket live.

// Les logs structurés gardent les erreurs d'enrichissement visibles.
use tracing::warn;

use crate::{
    app_state::AppState,
    features::{live::dto::LiveNewsItem, translation::domain::PersistedNewsTranslation},
};

/// Construit le payload live compact EN+FR à partir d'une traduction persistée.
pub(in crate::features::live::websocket::news) async fn build_live_news_item(
    state: &AppState,
    translation: PersistedNewsTranslation,
) -> LiveNewsItem {
    // On relit l'event source pour enrichir l'item diffusé sans dupliquer toutes ses colonnes
    // dans `news_translations`. C'est un compromis de schéma acceptable pour le MVP SQLite.
    let source_event = match state.news.get_news_by_id(translation.news_event_id).await {
        Ok(source_event) => source_event,
        Err(error) => {
            warn!(
                %error,
                news_event_id = translation.news_event_id,
                "failed to load source event for live websocket payload"
            );
            None
        }
    };

    LiveNewsItem::from_translation(
        translation,
        source_event.as_ref(),
        state.config.translation.max_body_chars,
    )
}
