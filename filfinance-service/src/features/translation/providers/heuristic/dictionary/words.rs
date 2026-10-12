//! Dictionnaire mot-a-mot du fallback heuristique francais.

mod early;
mod late;
mod middle;

/// Traduit le coeur d'un token deja extrait de sa ponctuation.
pub(super) fn translate_core(core: &str) -> &str {
    // Le `match` sur `to_ascii_lowercase()` donne un dictionnaire local simple et totalement deterministe.
    let normalized = core.to_ascii_lowercase();
    early::translate_early_word(&normalized)
        .or_else(|| middle::translate_middle_word(&normalized))
        .or_else(|| late::translate_late_word(&normalized))
        .unwrap_or(core)
}
