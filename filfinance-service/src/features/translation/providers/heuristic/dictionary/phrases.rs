//! Remplacements multi-mots prioritaires du dictionnaire heuristique.

/// Applique les expressions de marche avant la traduction mot a mot.
pub(super) fn apply_phrase_replacements(input: &str) -> String {
    let mut translated = input.to_owned();

    // Les remplacements de phrase passent avant le mot-a-mot pour conserver des expressions de marche.
    for (source, replacement) in phrase_replacements() {
        translated = replace_case_insensitive(&translated, source, replacement);
    }

    translated
}

/// Retourne des remplacements multi-mots prioritaires avant la traduction mot a mot.
fn phrase_replacements() -> [(&'static str, &'static str); 15] {
    [
        ("breaking news", "alerte"),
        ("rate cuts", "baisses de taux"),
        ("rate cut", "baisse de taux"),
        ("holds rates", "maintient les taux"),
        ("holds rate", "maintient le taux"),
        ("cut rates", "baisse les taux"),
        ("cuts rates", "baisse les taux"),
        ("treasury yields", "rendements obligataires"),
        ("bond yields", "rendements obligataires"),
        ("data dependent", "dependant des donnees"),
        ("risk-off", "aversion au risque"),
        ("risk-on", "retour de l'appetit pour le risque"),
        ("all time high", "plus haut historique"),
        ("year over year", "sur un an"),
        ("month over month", "sur un mois"),
    ]
}

/// Remplace une expression ASCII sans tenir compte de la casse.
fn replace_case_insensitive(input: &str, needle: &str, replacement: &str) -> String {
    if input.is_empty() || needle.is_empty() || !input.is_ascii() || !needle.is_ascii() {
        return input.to_owned();
    }

    let input_lower = input.to_ascii_lowercase();
    let needle_lower = needle.to_ascii_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut start = 0;

    while let Some(found) = input_lower[start..].find(&needle_lower) {
        let absolute = start + found;
        // On recopie la tranche precedente puis le remplacement, sans regex ni allocation exotique.
        result.push_str(&input[start..absolute]);
        result.push_str(replacement);
        start = absolute + needle.len();
    }

    result.push_str(&input[start..]);
    result
}
