//! Construction du draft de traduction persiste.

use crate::{
    config::TranslationConfig,
    features::{
        glossary::{
            domain::GlossaryEntry,
            pipeline::{apply_display_glossary, apply_source_glossary},
        },
        news::domain::PersistedNewsEvent,
        translation::{
            domain::{NewsTranslationDraft, TranslationDraftBuild, TranslationRequest},
            provider::TranslationProvider,
        },
    },
    shared::text::normalize_whitespace,
};

mod narration;
mod provider;
mod text;

use narration::build_narration_text;
use provider::translate_with_fallback;
use text::normalize_optional_text;

/// Construit la variante texte FR minimale a partir d'un evenement persiste.
pub(crate) async fn build_translation_draft(
    item: &PersistedNewsEvent,
    config: &TranslationConfig,
    provider: &mut TranslationProvider,
    glossary_entries: &[GlossaryEntry],
) -> Option<TranslationDraftBuild> {
    let source_title = normalize_whitespace(&item.title);
    if source_title.is_empty() {
        return None;
    }
    let title_glossary = apply_source_glossary(&source_title, glossary_entries);
    let source_body = normalize_optional_text(item.body.as_deref());
    let body_glossary = source_body
        .as_deref()
        .map(|body| apply_source_glossary(body, glossary_entries));

    // Le brouillon normalise des maintenant les longueurs et le couple langue source/cible.
    let request = TranslationRequest {
        source_title: title_glossary.text.clone(),
        source_body: body_glossary
            .as_ref()
            .map(|application| application.text.clone()),
        source_language: config.source_language.clone(),
        target_language: config.target_language.clone(),
    };

    let (provider_translation, fallback_detail) =
        translate_with_fallback(item, provider, &request).await;

    let translated_title = normalize_whitespace(
        &title_glossary
            .restore_display_text(&provider_translation.translated_title, glossary_entries),
    );
    let translated_body = provider_translation
        .translated_body
        .as_deref()
        .map(|body| {
            body_glossary
                .as_ref()
                .map(|application| application.restore_display_text(body, glossary_entries))
                .unwrap_or_else(|| apply_display_glossary(body, glossary_entries))
        })
        .and_then(|body| normalize_optional_text(Some(body).as_deref()));
    let translation_mode = if translated_title == source_title {
        // Si le titre n'a pas bouge, on assume que la meilleure sortie est une narration FR
        // tout en gardant le titre source intact pour le debug et la fidelite.
        "source_with_french_narration".to_owned()
    } else {
        provider_translation.translation_mode
    };

    let narration_text = build_narration_text(
        item,
        &item.source,
        &source_title,
        &translated_title,
        &translation_mode,
    );

    Some(TranslationDraftBuild {
        draft: NewsTranslationDraft {
            language: config.target_language.clone(),
            status: "ready".to_owned(),
            translated_title,
            translated_body,
            narration_text,
            translation_mode,
            error_message: fallback_detail.clone(),
        },
        fallback_detail,
        glossary_hits: title_glossary.hits
            + body_glossary
                .map(|application| application.hits)
                .unwrap_or(0),
    })
}
