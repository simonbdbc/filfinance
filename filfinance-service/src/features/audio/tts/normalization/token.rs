//! Regles TTS appliquees a un token isole.

mod market;
mod time;

use market::{
    normalize_central_bank_token, normalize_currency_token, normalize_fx_pair,
    normalize_macro_acronym_token, normalize_percentage_token, normalize_uppercase_token,
};
use time::normalize_time_token;

use super::helpers::split_token_parts;

/// Adapte un token unitaire a une prononciation plus stable.
pub(super) fn normalize_tts_token(token: &str, previous_core_lower: Option<&str>) -> String {
    let (prefix, core, suffix) = split_token_parts(token);
    if core.is_empty() {
        return token.to_owned();
    }

    let normalized_core = normalize_time_token(core)
        .or_else(|| normalize_percentage_token(core))
        .or_else(|| normalize_fx_pair(core))
        .or_else(|| normalize_currency_token(core))
        .or_else(|| normalize_central_bank_token(core, previous_core_lower))
        .or_else(|| normalize_macro_acronym_token(core))
        .or_else(|| normalize_uppercase_token(core))
        .unwrap_or_else(|| core.to_owned());

    format!("{prefix}{normalized_core}{suffix}")
}
