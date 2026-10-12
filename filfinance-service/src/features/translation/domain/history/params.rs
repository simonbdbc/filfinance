//! Parametres normalises de l'historique traduction.

use super::{
    filters::{normalize_day_filter_value, normalize_filter_value},
    scope::TranslationHistoryScope,
};

/// Parametres normalises utilises par le repository pour l'historique des traductions.
#[derive(Debug, Clone)]
pub struct TranslationHistoryParams {
    /// Limite bornee de la page.
    pub limit: i64,
    /// Source logique nettoyee.
    pub source: Option<String>,
    /// Jour cible valide au format `YYYY-MM-DD`.
    pub day: Option<String>,
    /// Scope fonctionnel des items retournes.
    pub history_scope: TranslationHistoryScope,
    /// Curseur `before_id` valide si strictement positif.
    pub before_id: Option<i64>,
}

impl TranslationHistoryParams {
    /// Construit des parametres internes pour l'historique traduit en bornant et nettoyant la query.
    pub fn new(
        limit: Option<i64>,
        source: Option<String>,
        day: Option<String>,
        history_scope: Option<String>,
        before_id: Option<i64>,
    ) -> Self {
        Self {
            // L'historique client journalier n'a pas besoin de pages enormes pour le MVP.
            limit: limit.unwrap_or(100).clamp(1, 500),
            source: normalize_filter_value(source),
            day: normalize_day_filter_value(day),
            history_scope: TranslationHistoryScope::from_query_value(history_scope),
            before_id: before_id.filter(|before_id| *before_id > 0),
        }
    }
}
