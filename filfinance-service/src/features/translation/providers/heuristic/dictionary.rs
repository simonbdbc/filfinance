//! Dictionnaire heuristique borne pour le fallback francais.

use crate::shared::text::{cleanup_spacing, normalize_whitespace, split_token_parts};

mod phrases;
mod words;

/// Traduit un texte court de news via un mapping heuristique borne.
pub(crate) fn heuristic_translate_text(text: &str) -> String {
    let translated = phrases::apply_phrase_replacements(&normalize_whitespace(text));

    let translated = translated
        .split_whitespace()
        .map(translate_token)
        .collect::<Vec<_>>()
        .join(" ");

    normalize_whitespace(&cleanup_spacing(&translated))
}

/// Traduit un token simple tout en preservant sa ponctuation adjacente.
fn translate_token(token: &str) -> String {
    let (prefix, core, suffix) = split_token_parts(token);
    if core.is_empty() {
        return token.to_owned();
    }

    let translated_core = words::translate_core(core);

    format!("{prefix}{translated_core}{suffix}")
}
