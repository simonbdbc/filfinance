//! Normalisation texte locale au draft de traduction.

use crate::shared::text::normalize_whitespace;

/// Normalise un texte optionnel sans le tronquer avant traduction/persistance.
pub(super) fn normalize_optional_text(text: Option<&str>) -> Option<String> {
    // Les limites compactes appartiennent aux DTO live, pas au contrat persiste.
    text.map(normalize_whitespace)
        .filter(|value| !value.is_empty())
}
