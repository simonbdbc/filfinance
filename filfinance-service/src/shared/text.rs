//! Helpers texte transverses conserves hors des domaines metier.

/// Tronque un texte en gardant un resultat stable pour la lecture cote client.
pub(crate) fn trim_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

    // `saturating_sub(1)` evite un underflow si `max_chars` vaut 0 ou 1.
    let trimmed = text
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{}…", trimmed.trim_end())
}

/// Uniformise les espaces pour le stockage, les providers et la narration.
pub(crate) fn normalize_whitespace(text: &str) -> String {
    // Cette approche est volontairement simple et Unicode-safe pour les espaces.
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Separe ponctuation et coeur de mot pour eviter de perdre les signes adjacents.
pub(crate) fn split_token_parts(token: &str) -> (&str, &str, &str) {
    // La fonction retourne trois `&str` empruntes dans la chaine d'origine: zero allocation.
    let start = token
        .find(|ch: char| ch.is_ascii_alphanumeric())
        .unwrap_or(token.len());
    let end = token
        .rfind(|ch: char| ch.is_ascii_alphanumeric())
        .map(|index| index + 1)
        .unwrap_or(start);

    (&token[..start], &token[start..end], &token[end..])
}

/// Nettoie les espaces parasites autour d'une ponctuation simple.
pub(crate) fn cleanup_spacing(text: &str) -> String {
    // Le petit tableau statique evite de multiplier des appels ad hoc disperses dans le code.
    [
        (" ,", ","),
        (" .", "."),
        (" :", ":"),
        (" ;", ";"),
        (" !", "!"),
        (" ?", "?"),
        ("  ", " "),
    ]
    .into_iter()
    .fold(text.to_owned(), |acc, (from, to)| acc.replace(from, to))
}
