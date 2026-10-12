//! Normalisation des anciens préfixes de narration persistés.

use super::super::event::PersistedNewsEvent;
use super::prefix::{build_news_clock_label, build_news_narration_prefix};

/// Normalise une narration persistee pour appliquer le prefixe canonique derive de l'evenement source.
pub fn normalize_news_narration_prefix(
    narration_text: String,
    source_event: Option<&PersistedNewsEvent>,
) -> String {
    let Some(source_event) = source_event else {
        // Sans evenement source, le DTO ne peut pas recalculer l'heure ni le niveau: on preserve le texte.
        return narration_text;
    };

    // `trim_start()` evite qu'une indentation accidentelle empeche la detection du prefixe existant.
    let trimmed_narration = narration_text.trim_start();
    // Le prefixe attendu vient du meme helper que la creation initiale de narration.
    let expected_prefix = build_news_narration_prefix(source_event);
    // Le marqueur inclut le point final pour reconnaitre une narration deja canonicalisee.
    let expected_marker = format!("{expected_prefix}.");

    if trimmed_narration.starts_with(&expected_marker) {
        // La fonction reste idempotente: un texte deja correct ne doit pas etre modifie.
        return trimmed_narration.to_owned();
    }

    // Les anciennes narrations ont souvent commence directement par `HH:MM. ...`, avant l'ajout
    // du libelle metier explicite. On repare ce cas pour que le texte et l'intro audio partagent
    // strictement le meme prefixe.
    normalize_legacy_narration_prefix(trimmed_narration, source_event, &expected_prefix)
}

/// Repare les variantes historiques vers le prefixe canonique calcule depuis l'event source.
fn normalize_legacy_narration_prefix(
    trimmed_narration: &str,
    source_event: &PersistedNewsEvent,
    expected_prefix: &str,
) -> String {
    // Le label horaire source permet de reconnaitre le format historique `HH:MM. texte`.
    let clock_label = build_news_clock_label(source_event);
    // Le marqueur historique porte l'heure seule, avant l'introduction du libelle metier.
    let clock_marker = format!("{clock_label}. ");

    if let Some(rest) = trimmed_narration.strip_prefix(&clock_marker) {
        // Cette branche repare le format historique `HH:MM. texte` et retire les anciens libelles
        // pour eviter `Nouvelle information. Nouvelle information`.
        let rest_without_legacy_label = strip_legacy_narration_label(rest);
        // La reconstruction remet toujours le libelle metier apres l'heure.
        return format!("{expected_prefix}. {rest_without_legacy_label}");
    }

    let legacy_important_clock_marker = format!("Important {clock_label}. ");
    if let Some(rest) = trimmed_narration.strip_prefix(&legacy_important_clock_marker) {
        // Cette branche repare l'ancien mauvais ordre `Important HH:MM. texte`.
        return format!("{expected_prefix}. {rest}");
    }

    // Dernier recours: si aucune heure n'est detectee, on prefixe quand meme avec le marqueur canonique.
    format!("{expected_prefix}. {trimmed_narration}")
}

/// Retire les libelles transitoires deja presents apres `HH:MM.` avant de reconstruire le prefixe.
fn strip_legacy_narration_label(rest: &str) -> &str {
    rest.strip_prefix("Important. ")
        .or_else(|| rest.strip_prefix("Information importante. "))
        .or_else(|| rest.strip_prefix("Nouvelle information. "))
        .unwrap_or(rest)
}
