//! Helpers finance pour la normalisation TTS.

/// Detecte les unites anglaises usuelles des points de base.
pub(in crate::features::audio::tts::normalization) fn is_basis_point_unit(core: &str) -> bool {
    matches!(core, "bp" | "bps" | "Bp" | "Bps" | "BP" | "BPS")
}

/// Parse les tenors obligataires ecrits comme `10Y`.
pub(in crate::features::audio::tts::normalization) fn parse_year_tenor(core: &str) -> Option<u32> {
    let years = core.strip_suffix('Y').or_else(|| core.strip_suffix('y'))?;
    if years.is_empty() || !years.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }

    years.parse::<u32>().ok().filter(|years| *years > 0)
}

/// Nomme les principales devises utilisees dans les titres macro et FX.
pub(in crate::features::audio::tts::normalization) fn currency_code_spoken_label(
    code: &str,
) -> Option<String> {
    let label = match code {
        "EUR" => "euro",
        "USD" => "dollar",
        "GBP" => "livre sterling",
        "JPY" => "yen",
        "CHF" => "franc suisse",
        "CAD" => "dollar canadien",
        "AUD" => "dollar australien",
        "NZD" => "dollar néo-zélandais",
        "CNY" | "CNH" => "yuan",
        _ => return None,
    };
    Some(label.to_owned())
}
