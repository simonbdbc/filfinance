//! Mapping des traductions persistantes vers le message edge.

// On importe les DTO edge et les modeles persistants utiles au live.
use crate::{
    features::live::dto::{EdgeLiveMessage, EdgeTranslatedNewsItemPayload, LiveNewsItem},
    features::translation::domain::PersistedNewsTranslation,
    shared::time::now_rfc3339,
};

/// Verifie qu'une traduction appartient bien au flux live public.
pub(crate) fn is_publishable_translation(
    translation: &PersistedNewsTranslation,
    target_language: &str,
) -> bool {
    // On normalise en ASCII car les statuts/langues du service sont des identifiants techniques.
    translation.language.eq_ignore_ascii_case(target_language)
        && translation.status.eq_ignore_ascii_case("ready")
}

/// Construit un identifiant monotone lisible depuis l'identifiant SQLite de traduction.
pub(crate) fn edge_event_id_for_translation_id(translation_id: i64) -> String {
    // `max(0)` evite un format surprenant dans les fixtures tout en gardant les IDs reels inchanges.
    format!("translated_news_item:{:020}", translation_id.max(0))
}

/// Construit le message edge `translated_news_item` a partir du DTO live existant.
pub(crate) fn build_translated_news_item_message(
    event_id: String,
    item: LiveNewsItem,
) -> EdgeLiveMessage {
    EdgeLiveMessage::TranslatedNewsItem {
        // `Box` conserve la meme forme memoire que le contrat edge declare dans `models`.
        payload: Box::new(EdgeTranslatedNewsItemPayload {
            event_id,
            server_time: now_rfc3339(),
            item,
        }),
    }
}
