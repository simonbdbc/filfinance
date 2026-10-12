//! Regles marche et sigles appliquees a un token TTS.

use super::super::helpers::{
    currency_code_spoken_label, is_uppercase_code, normalize_signed_number, spell_out_letters,
};

/// Rend les pourcentages plus naturels pour la synthese vocale.
pub(super) fn normalize_percentage_token(core: &str) -> Option<String> {
    let number = core.strip_suffix('%')?;
    Some(format!("{} pour cent", normalize_signed_number(number)?))
}

/// Rend une paire de devises plus intelligible qu'un slash literal.
pub(super) fn normalize_fx_pair(core: &str) -> Option<String> {
    let (left, right) = core.split_once('/')?;
    if !is_uppercase_code(left) || !is_uppercase_code(right) {
        return None;
    }

    let left = currency_code_spoken_label(left).unwrap_or_else(|| spell_out_letters(left));
    let right = currency_code_spoken_label(right).unwrap_or_else(|| spell_out_letters(right));
    Some(format!("{left} {right}"))
}

/// Rend les devises courantes plus naturelles quand elles apparaissent seules.
pub(super) fn normalize_currency_token(core: &str) -> Option<String> {
    currency_code_spoken_label(core)
}

/// Traite les banques centrales que Piper lit souvent comme des mots ou des lettres maladroites.
pub(super) fn normalize_central_bank_token(
    core: &str,
    previous_core_lower: Option<&str>,
) -> Option<String> {
    if core == "Fed" || core == "FED" {
        // Si le texte contient deja `la Fed`, on evite de doubler l'article.
        if previous_core_lower == Some("la") {
            return Some("Fed".to_owned());
        }
        return Some("la Fed".to_owned());
    }

    match core {
        "ECB" | "BCE" => Some("Banque centrale européenne".to_owned()),
        "BoE" | "BOE" => Some("Banque d'Angleterre".to_owned()),
        "BoJ" | "BOJ" => Some("Banque du Japon".to_owned()),
        "PBoC" | "PBOC" => Some("Banque populaire de Chine".to_owned()),
        _ => None,
    }
}

/// Remplace quelques sigles macro par une forme plus claire que l'epellation brute.
pub(super) fn normalize_macro_acronym_token(core: &str) -> Option<String> {
    let label = match core {
        "NFP" => "rapport emploi américain",
        "GDP" => "P I B",
        "CPI" => "C P I",
        "PCE" => "P C E",
        "PMI" => "P M I",
        "ISM" => "I S M",
        "FOMC" => "F O M C",
        "JOLTS" => "rapport JOLTS",
        "US" => "américain",
        _ => return None,
    };
    Some(label.to_owned())
}

/// Epelle les sigles courts pour aider la voix systeme.
pub(super) fn normalize_uppercase_token(core: &str) -> Option<String> {
    if !is_uppercase_code(core) {
        return None;
    }

    Some(spell_out_letters(core))
}
