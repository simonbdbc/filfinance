//! Normalisation locale des filtres d'historique traduction.

/// Nettoie un filtre texte de query string pour les comparaisons repository.
pub(super) fn normalize_filter_value(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        // Une chaine vide n'a aucun sens comme filtre et redevient donc `None`.
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

/// Valide un jour de filtre au format simple `YYYY-MM-DD`.
pub(super) fn normalize_day_filter_value(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        let is_valid = trimmed.len() == 10
            && trimmed.chars().enumerate().all(|(index, ch)| match index {
                4 | 7 => ch == '-',
                _ => ch.is_ascii_digit(),
            });

        if is_valid {
            Some(trimmed.to_owned())
        } else {
            None
        }
    })
}
