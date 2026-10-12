//! Helpers numériques pour la normalisation TTS.

/// Convertit un nombre signe en formulation orale stable.
pub(in crate::features::audio::tts::normalization) fn normalize_signed_number(
    value: &str,
) -> Option<String> {
    if !is_numeric_like(value) {
        return None;
    }

    if let Some(rest) = value.strip_prefix('+') {
        return Some(format!("plus {}", normalize_unsigned_number(rest)));
    }

    if let Some(rest) = value.strip_prefix('-') {
        return Some(format!("moins {}", normalize_unsigned_number(rest)));
    }

    Some(normalize_unsigned_number(value))
}

/// Rend les decimales plus naturelles pour une voix francaise.
fn normalize_unsigned_number(value: &str) -> String {
    value.replace('.', " virgule ").replace(',', " virgule ")
}

/// Convertit les nombres horaires et les minutes courantes en mots francais simples.
pub(in crate::features::audio::tts::normalization) fn french_number_word(
    value: u32,
) -> Option<String> {
    let label = match value {
        0 => "zéro",
        1 => "un",
        2 => "deux",
        3 => "trois",
        4 => "quatre",
        5 => "cinq",
        6 => "six",
        7 => "sept",
        8 => "huit",
        9 => "neuf",
        10 => "dix",
        11 => "onze",
        12 => "douze",
        13 => "treize",
        14 => "quatorze",
        15 => "quinze",
        16 => "seize",
        17 => "dix sept",
        18 => "dix huit",
        19 => "dix neuf",
        20 => "vingt",
        30 => "trente",
        40 => "quarante",
        50 => "cinquante",
        _ => return french_compound_number_word(value),
    };
    Some(label.to_owned())
}

/// Compose les nombres francais entre 21 et 59 sans viser une grammaire generale.
fn french_compound_number_word(value: u32) -> Option<String> {
    let tens = (value / 10) * 10;
    let unit = value % 10;
    if !(20..=59).contains(&value) || unit == 0 {
        return None;
    }

    let tens_label = french_number_word(tens)?;
    let unit_label = french_number_word(unit)?;
    if unit == 1 {
        Some(format!("{tens_label} et {unit_label}"))
    } else {
        Some(format!("{tens_label} {unit_label}"))
    }
}

/// Ajuste `une` pour les minutes tout en gardant le reste du dictionnaire numerique commun.
pub(in crate::features::audio::tts::normalization) fn french_minute_word(
    value: u32,
) -> Option<String> {
    if value == 1 {
        Some("une".to_owned())
    } else {
        french_number_word(value)
    }
}

/// Detecte un nombre simple utilisable dans les narrations de marche.
pub(in crate::features::audio::tts::normalization) fn is_numeric_like(value: &str) -> bool {
    let candidate = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    !candidate.is_empty()
        && candidate
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == ',')
}
