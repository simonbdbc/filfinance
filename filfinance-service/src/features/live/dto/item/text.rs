//! Helpers texte bornant la taille des payloads live.

/// Tronque un texte libre pour les payloads live afin de garder des frames WebSocket compactes.
pub(super) fn truncate_text(value: String, limit: usize) -> String {
    // Si le texte est deja assez court, on le retourne sans allocation supplementaire.
    if value.chars().count() <= limit {
        return value;
    }

    // `saturating_sub(1)` evite un underflow si `limit` vaut 0.
    let truncated = value
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>();
    // On ajoute une ellipsis Unicode pour signaler explicitement la troncature.
    format!("{}…", truncated.trim_end())
}

/// Tronque un champ texte optionnel sans casser la semantique `None`.
pub(super) fn truncate_optional_text(value: Option<String>, limit: usize) -> Option<String> {
    // `map(...)` applique la troncature seulement si une valeur existe deja.
    value.map(|value| truncate_text(value, limit))
}
