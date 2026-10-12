//! Appel du provider de traduction et fallback heuristique.

use tracing::warn;

use crate::features::{
    news::domain::PersistedNewsEvent,
    translation::{
        domain::{ProviderTranslation, TranslationRequest},
        provider::TranslationProvider,
        providers::heuristic::HeuristicTranslationProvider,
    },
};

/// Appelle le provider principal, puis bascule sur l'heuristique si le downstream echoue.
pub(super) async fn translate_with_fallback(
    item: &PersistedNewsEvent,
    provider: &mut TranslationProvider,
    request: &TranslationRequest,
) -> (ProviderTranslation, Option<String>) {
    match provider.translate(request).await {
        Ok(translated) => (translated, None),
        Err(error) => {
            warn!(
                %error,
                provider = provider.provider_name(),
                news_event_id = item.id,
                source = %item.source,
                "translation provider failed; falling back to heuristic translation"
            );

            // Le fallback heuristique garantit qu'une news ne bloque pas tout le flux FR.
            let fallback = HeuristicTranslationProvider.translate(request);
            let fallback_mode = format!("heuristic_fallback_after_{}", provider.provider_name());
            (
                ProviderTranslation {
                    translation_mode: fallback_mode,
                    ..fallback
                },
                Some(error.to_string()),
            )
        }
    }
}
