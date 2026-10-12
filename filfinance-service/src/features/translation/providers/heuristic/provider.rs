//! Provider heuristique de secours pour les traductions embarquees.

use crate::features::translation::{
    domain::{ProviderTranslation, TranslationRequest},
    provider::HEURISTIC_PROVIDER,
};

use super::{dictionary::heuristic_translate_text, protection::protect_finance_tokens};

/// Provider heuristique embarque servant de fallback garanti.
#[derive(Debug, Clone, Default)]
pub(crate) struct HeuristicTranslationProvider;

impl HeuristicTranslationProvider {
    /// Traduit localement un texte court a cout nul, sans dependance externe.
    pub(crate) fn translate(&self, request: &TranslationRequest) -> ProviderTranslation {
        // On protege d'abord les tokens finance (tickers, pourcentages, paires, etc.) pour
        // limiter les traductions destructrices sur des valeurs qui doivent rester exactes.
        let protected_title = protect_finance_tokens(&request.source_title);
        let translated_title =
            protected_title.restore(&heuristic_translate_text(&protected_title.masked_text));

        let translated_body = request.source_body.as_deref().map(|body| {
            let protected_body = protect_finance_tokens(body);
            protected_body.restore(&heuristic_translate_text(&protected_body.masked_text))
        });

        ProviderTranslation {
            translated_title,
            translated_body,
            translation_mode: HEURISTIC_PROVIDER.to_owned(),
        }
    }
}
