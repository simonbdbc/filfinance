//! Parametres et reponses de pagination du flux news.

// Serialize et Deserialize gardent le contrat JSON historique des reponses HTTP.
use serde::{Deserialize, Serialize};

use super::event::PersistedNewsEvent;

/// Parametres normalises utilises par le repository pour les lectures historiques.
#[derive(Debug, Clone)]
pub struct NewsListParams {
    /// Limite normalisee et bornee.
    pub limit: i64,
    /// Filtre source nettoye.
    pub source: Option<String>,
    /// Filtre event_kind nettoye.
    pub event_kind: Option<String>,
    /// Curseur `before_id` valide si strictement positif.
    pub before_id: Option<i64>,
}

impl NewsListParams {
    /// Construit des parametres internes en appliquant les bornes et nettoyages MVP.
    pub fn new(
        limit: Option<i64>,
        source: Option<String>,
        event_kind: Option<String>,
        before_id: Option<i64>,
    ) -> Self {
        Self {
            // La limite reste volontairement bornee pour garder une API de debug legere.
            limit: limit.unwrap_or(100).clamp(1, 500),
            // `normalize_filter_value` evite les espaces parasites et harmonise la casse.
            source: normalize_filter_value(source),
            event_kind: normalize_filter_value(event_kind),
            // `filter(...)` applique ici une petite validation declarative sur l'`Option<i64>`.
            before_id: before_id.filter(|before_id| *before_id > 0),
        }
    }
}

/// Payload HTTP renvoye pour les lectures paginees de l'historique.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsListResponse {
    /// Liste des items de la page courante.
    pub items: Vec<PersistedNewsEvent>,
    /// Indique si une page suivante existe.
    pub has_more: bool,
    /// Curseur a renvoyer au prochain appel si `has_more` est vrai.
    pub next_before_id: Option<i64>,
}

impl NewsListResponse {
    /// Construit une page de resultat repository a partir des items retournes.
    pub fn new(
        items: Vec<PersistedNewsEvent>,
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

/// Nettoie un filtre texte de query string pour les comparaisons repository.
fn normalize_filter_value(value: Option<String>) -> Option<String> {
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
