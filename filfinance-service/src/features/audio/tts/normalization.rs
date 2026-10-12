//! Normalisation de narration avant synthese vocale.

mod helpers;
mod sequences;
mod token;

use helpers::{core_lower, last_core_lower};
use sequences::normalize_tts_token_sequence;
use token::normalize_tts_token;

/// Normalise une narration FR pour une lecture TTS plus intelligible.
pub(crate) fn normalize_narration_text_for_tts(text: &str) -> String {
    // Le normaliseur travaille sur les mots affiches sans modifier le texte persiste ou diffuse au client.
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    let mut normalized_tokens = Vec::with_capacity(tokens.len());
    let mut previous_core_lower: Option<String> = None;
    let mut index = 0usize;

    while index < tokens.len() {
        if let Some((normalized_sequence, consumed_tokens)) =
            normalize_tts_token_sequence(&tokens[index..])
        {
            // Les regles multi-tokens couvrent les cas finance qui perdent leur sens si on traite mot par mot.
            previous_core_lower = last_core_lower(&tokens[index..index + consumed_tokens]);
            normalized_tokens.push(normalized_sequence);
            index += consumed_tokens;
            continue;
        }

        let normalized_token = normalize_tts_token(tokens[index], previous_core_lower.as_deref());
        previous_core_lower = core_lower(tokens[index]);
        normalized_tokens.push(normalized_token);
        index += 1;
    }

    normalized_tokens.join(" ")
}
