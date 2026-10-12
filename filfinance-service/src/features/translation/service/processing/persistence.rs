//! Persistence des traductions prêtes.

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        news::domain::PersistedNewsEvent,
        translation::domain::{PersistedNewsTranslation, TranslationDraftBuild},
    },
};

/// Upsert la traduction calculée et renvoie la projection persistée.
pub(super) async fn persist_translation(
    state: &AppState,
    item: &PersistedNewsEvent,
    outcome: &TranslationDraftBuild,
) -> Result<PersistedNewsTranslation, AppError> {
    // Le store reste la source de vérité: on lui confie l'idempotence `(news_event_id, language)`.
    state.news.upsert_translation(item, &outcome.draft).await
}
