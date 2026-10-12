//! Scope fonctionnel de l'historique traduction.

use super::filters::normalize_filter_value;

/// Portee fonctionnelle demandee pour l'historique des traductions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationHistoryScope {
    /// Retourne sans distinction les items reels et les runs de test.
    All,
    /// Retourne seulement les items reels du feed.
    RealFeed,
    /// Retourne seulement les items synthetiques de test.
    TestRun,
}

impl TranslationHistoryScope {
    /// Interprete une valeur de query string en scope metier robuste.
    pub fn from_query_value(value: Option<String>) -> Self {
        match normalize_filter_value(value).as_deref() {
            Some("real_feed") => Self::RealFeed,
            Some("test_run") => Self::TestRun,
            _ => Self::All,
        }
    }
}
