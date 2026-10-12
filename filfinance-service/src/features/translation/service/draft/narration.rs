//! Construction de la narration FR persistée avec une traduction.

use crate::features::news::domain::{PersistedNewsEvent, build_news_narration_prefix};

/// Construit un texte de narration stable pour l'audio et Telegram.
pub(super) fn build_narration_text(
    item: &PersistedNewsEvent,
    source: &str,
    source_title: &str,
    translated_title: &str,
    translation_mode: &str,
) -> String {
    // Le modele centralise les marqueurs `HH:MM` et libelle metier pour eviter un drift entre sorties.
    let narration_prefix = build_news_narration_prefix(item);

    if translation_mode == "source_with_french_narration" {
        // Ce cas rend explicite que le titre visible reste anglais mais que la narration reste francaise.
        format!(
            "{}. Source {}. Titre original en anglais. {}.",
            narration_prefix,
            humanize_source_name(source),
            source_title
        )
    } else {
        format!("{}. {}.", narration_prefix, translated_title)
    }
}

/// Rend le nom de source plus naturel dans la narration.
fn humanize_source_name(source: &str) -> &str {
    match source {
        "financial_juice" => "Financial Juice",
        "x" => "X",
        _ => source,
    }
}
