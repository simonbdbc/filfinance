//! Reponse HTTP de l'historique live traduit.

use serde::{Deserialize, Serialize};

use super::types::LiveNewsItem;

/// Payload HTTP renvoye au client pour l'historique EN+FR pret a afficher et a rejouer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveNewsHistoryResponse {
    /// Items live de la page courante.
    pub items: Vec<LiveNewsItem>,
    /// Indique si une page suivante existe.
    pub has_more: bool,
    /// Curseur de page suivante.
    pub next_before_id: Option<i64>,
}

impl LiveNewsHistoryResponse {
    /// Construit la reponse HTTP finale de l'historique traduit.
    pub fn new(items: Vec<LiveNewsItem>, has_more: bool, next_before_id: Option<i64>) -> Self {
        Self {
            items,
            has_more,
            next_before_id,
        }
    }
}
