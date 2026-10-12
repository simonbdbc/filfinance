//! Projection persistee et exposee d'un evenement news.

// Serialize et Deserialize gardent le payload HTTP identique, FromRow garde le mapping SQLx.
use serde::{Deserialize, Serialize};

use crate::features::news::domain::test_refs::extract_test_run_id;

/// Projection du modele metier vers la forme stockee et exposee.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersistedNewsEvent {
    /// Cle primaire SQLite.
    pub id: i64,
    /// Source logique de l'evenement.
    pub source: String,
    /// Type metier aplati en `String` pour la persistence.
    pub event_kind: String,
    /// Titre source persiste.
    pub title: String,
    /// URL source persistee.
    pub url: String,
    /// Date de publication source.
    pub published_at: Option<String>,
    /// Heure evenementielle source si distincte.
    pub event_time: Option<String>,
    /// Horodatage d'ingestion locale.
    pub ingested_at: String,
    /// Corps source optionnel.
    pub body: Option<String>,
    /// Niveau source optionnel conserve tel quel pour les clients et les decisions de priorisation.
    pub news_level: Option<String>,
    /// Importance macro optionnelle.
    pub importance: Option<String>,
    /// Pays macro optionnel.
    pub country_code: Option<String>,
    /// Devise macro optionnelle.
    pub currency: Option<String>,
    /// Valeur macro publiee.
    pub actual_value: Option<String>,
    /// Valeur macro attendue.
    pub forecast_value: Option<String>,
    /// Valeur macro precedente.
    pub previous_value: Option<String>,
    /// Reference brute de debug/source.
    pub raw_payload_ref: Option<String>,
    /// Signature de dedup persistee.
    pub signature: String,
}

impl PersistedNewsEvent {
    /// Retourne l'identifiant de run de test s'il s'agit d'un evenement synthetique E2E.
    pub fn test_run_id(&self) -> Option<&str> {
        extract_test_run_id(self.raw_payload_ref.as_deref())
    }
}
