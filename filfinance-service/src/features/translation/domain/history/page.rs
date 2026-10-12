//! Page repository des traductions persistentes.

use crate::features::translation::domain::PersistedNewsTranslation;

/// Page repository des traductions persistentes avant projection API finale.
#[derive(Debug, Clone)]
pub struct TranslationHistoryPage {
    /// Lignes de traduction de la page courante.
    pub items: Vec<PersistedNewsTranslation>,
    /// Indique si une page suivante existe.
    pub has_more: bool,
    /// Curseur a renvoyer au prochain appel si `has_more` est vrai.
    pub next_before_id: Option<i64>,
}

impl TranslationHistoryPage {
    /// Construit une page repository de traductions.
    pub fn new(
        items: Vec<PersistedNewsTranslation>,
        has_more: bool,
        next_before_id: Option<i64>,
    ) -> Self {
        Self {
            items,
            has_more,
            next_before_id,
        }
    }
}
