//! Helpers de découpe et lecture des tokens TTS.

/// Retourne le noyau d'un token en minuscules pour les regles dependantes du contexte.
pub(in crate::features::audio::tts::normalization) fn core_lower(token: &str) -> Option<String> {
    let (_, core, _) = split_token_parts(token);
    if core.is_empty() {
        None
    } else {
        Some(core.to_lowercase())
    }
}

/// Retourne le dernier noyau significatif d'une sequence consommee.
pub(in crate::features::audio::tts::normalization) fn last_core_lower(
    tokens: &[&str],
) -> Option<String> {
    tokens.iter().rev().find_map(|token| core_lower(token))
}

/// Detecte un code compose uniquement de lettres majuscules ASCII.
pub(in crate::features::audio::tts::normalization) fn is_uppercase_code(value: &str) -> bool {
    let len = value.chars().count();
    (2..=5).contains(&len) && value.chars().all(|ch| ch.is_ascii_uppercase())
}

/// Separe ponctuation d'enrobage et noyau semantique du token.
pub(in crate::features::audio::tts::normalization) fn split_token_parts(
    token: &str,
) -> (&str, &str, &str) {
    let start = token
        .char_indices()
        .find(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '+' | '-' | '%' | '/'))
        .map(|(index, _)| index)
        .unwrap_or(token.len());
    let end = token
        .char_indices()
        .rev()
        .find(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '+' | '-' | '%' | '/'))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(start);

    (&token[..start], &token[start..end], &token[end..])
}

/// Transforme `NVDA` en `N V D A` pour la voix systeme.
pub(in crate::features::audio::tts::normalization) fn spell_out_letters(value: &str) -> String {
    value
        .chars()
        .map(|ch| ch.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}
