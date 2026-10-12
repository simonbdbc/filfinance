//! Regles TTS qui consomment plusieurs tokens consecutifs.

use super::helpers::{
    is_basis_point_unit, is_numeric_like, normalize_signed_number, parse_year_tenor,
    split_token_parts,
};

/// Normalise les expressions finance composees de plusieurs tokens.
pub(super) fn normalize_tts_token_sequence(tokens: &[&str]) -> Option<(String, usize)> {
    // `25 bps` et `US 10Y` sont les deux cas ou le sens vient explicitement du token suivant.
    normalize_basis_points_sequence(tokens).or_else(|| normalize_us_treasury_tenor_sequence(tokens))
}

/// Transforme `25 bps` en `25 points de base` en preservant la ponctuation finale.
fn normalize_basis_points_sequence(tokens: &[&str]) -> Option<(String, usize)> {
    let [current_token, next_token, ..] = tokens else {
        return None;
    };
    let (prefix, core, suffix) = split_token_parts(current_token);
    let (_, next_core, next_suffix) = split_token_parts(next_token);
    if !suffix.is_empty() || !is_numeric_like(core) || !is_basis_point_unit(next_core) {
        return None;
    }

    let signed_number = normalize_signed_number(core)?;
    Some((
        format!("{prefix}{signed_number} points de base{next_suffix}"),
        2,
    ))
}

/// Transforme `US 10Y` en une expression obligataire francaise stable pour Piper.
fn normalize_us_treasury_tenor_sequence(tokens: &[&str]) -> Option<(String, usize)> {
    let [current_token, next_token, ..] = tokens else {
        return None;
    };
    let (prefix, core, suffix) = split_token_parts(current_token);
    let (_, next_core, next_suffix) = split_token_parts(next_token);
    if !suffix.is_empty() || !core.eq_ignore_ascii_case("US") {
        return None;
    }

    let years = parse_year_tenor(next_core)?;
    Some((
        format!("{prefix}Treasury américain à {years} ans{next_suffix}"),
        2,
    ))
}
