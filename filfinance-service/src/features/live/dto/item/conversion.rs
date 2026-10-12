//! Conversion d'une traduction persistee vers le contrat live user-facing.

use crate::features::{
    news::domain::{PersistedNewsEvent, normalize_news_narration_prefix},
    translation::domain::PersistedNewsTranslation,
};

use super::{
    text::{truncate_optional_text, truncate_text},
    types::LiveNewsItem,
};

impl LiveNewsItem {
    /// Construit un payload compact EN+FR a partir d'une traduction persistee et de son event source.
    pub fn from_translation(
        translation: PersistedNewsTranslation,
        source_event: Option<&PersistedNewsEvent>,
        body_limit: usize,
    ) -> Self {
        Self::from_translation_with_limit(translation, source_event, Some(body_limit))
    }

    /// Construit un payload EN+FR complet pour les lectures HTTP historiques ou detaillees.
    pub fn from_translation_full_text(
        translation: PersistedNewsTranslation,
        source_event: Option<&PersistedNewsEvent>,
    ) -> Self {
        Self::from_translation_with_limit(translation, source_event, None)
    }

    fn from_translation_with_limit(
        translation: PersistedNewsTranslation,
        source_event: Option<&PersistedNewsEvent>,
        body_limit: Option<usize>,
    ) -> Self {
        // On choisit un horodatage metier prioritaire:
        // 1. `published_at` de la traduction si deja calcule
        // 2. `published_at` source
        // 3. `event_time` source
        // 4. `ingested_at` source
        // 5. `created_at` de la traduction en dernier recours
        let timestamp = translation
            .published_at
            .clone()
            .or_else(|| {
                source_event.and_then(|event| {
                    event
                        .published_at
                        .as_ref()
                        .or(event.event_time.as_ref())
                        .cloned()
                        .or_else(|| Some(event.ingested_at.clone()))
                })
            })
            .unwrap_or_else(|| translation.created_at.clone());
        // `map(...)` sur `Option<&PersistedNewsEvent>` garde naturellement le caractere optionnel de l'URL.
        let url = source_event.map(|event| event.url.clone());
        // Le niveau reste porte par `news_events`; on le recopie seulement dans le DTO client final.
        let news_level = source_event.and_then(|event| event.news_level.clone());
        // Certaines traductions anciennes existent avant l'arrivée du `Level`; le DTO normalise donc les marqueurs au dernier moment.
        let narration_text =
            normalize_news_narration_prefix(translation.narration_text, source_event);
        let source_body = match body_limit {
            Some(limit) => truncate_optional_text(translation.source_body, limit),
            None => translation.source_body,
        };
        let translated_body = match body_limit {
            Some(limit) => truncate_optional_text(translation.translated_body, limit),
            None => translation.translated_body,
        };
        let narration_text = match body_limit {
            Some(limit) => truncate_text(narration_text, limit),
            None => narration_text,
        };

        Self {
            id: translation.id,
            news_event_id: translation.news_event_id,
            language: translation.language,
            status: translation.status,
            source: translation.source,
            event_kind: translation.event_kind,
            source_title: translation.source_title,
            // Les textes optionnels sont tronques pour garder des frames WS compactes.
            source_body,
            translated_title: translation.translated_title,
            translated_body,
            narration_text,
            translation_mode: translation.translation_mode,
            error_message: translation.error_message,
            news_level,
            // Le run de test est derive de la reference brute de l'evenement source si elle existe.
            test_run_id: source_event
                .and_then(|event| event.test_run_id())
                .map(ToOwned::to_owned),
            url,
            timestamp,
            created_at: translation.created_at,
            updated_at: translation.updated_at,
        }
    }
}
