//! Heuristique de nommage des champs de priorite FinancialJuice.

/// Heuristique volontairement large pour detecter `critical`, `breaking` ou un champ voisin.
pub(super) fn is_priority_like_unknown_key(key: &str) -> bool {
    // La comparaison insensible a la casse couvre les conventions JSON variees de FinancialJuice.
    let normalized_key = key.to_ascii_lowercase();

    // Ces fragments couvrent les noms probables sans imposer de connaitre le contrat exact a l'avance.
    [
        "critical",
        "breaking",
        "urgent",
        "priority",
        "importance",
        "important",
        "severity",
        "impact",
        "impid",
        "alert",
        "flash",
        "marketmoving",
        "market_moving",
        "strong",
    ]
    .iter()
    .any(|candidate| normalized_key.contains(candidate))
}
