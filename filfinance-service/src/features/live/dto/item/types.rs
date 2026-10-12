//! Contrat compact d'une news traduite diffusee en live.

use serde::{Deserialize, Serialize};

/// Payload compact diffuse vers les clients temps reel et reutilisable pour Telegram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveNewsItem {
    /// Identifiant de la traduction persistente.
    pub id: i64,
    /// Identifiant de l'evenement source.
    pub news_event_id: i64,
    /// Langue du payload derive.
    pub language: String,
    /// Statut de traitement de la traduction.
    pub status: String,
    /// Source logique de l'evenement.
    pub source: String,
    /// Type d'evenement aplati pour les clients.
    pub event_kind: String,
    /// Titre source EN.
    pub source_title: String,
    /// Corps source tronque pour le flux live.
    pub source_body: Option<String>,
    /// Titre FR derive.
    pub translated_title: String,
    /// Corps FR tronque.
    pub translated_body: Option<String>,
    /// Narration FR tronquee.
    pub narration_text: String,
    /// Provider ou mode de traduction utilise.
    pub translation_mode: String,
    /// Message d'erreur optionnel.
    pub error_message: Option<String>,
    /// Niveau source optionnel expose dans le flux live sans denormaliser `news_translations`.
    pub news_level: Option<String>,
    /// Identifiant de run de test si l'item est synthetique.
    pub test_run_id: Option<String>,
    /// URL source rattachee si disponible.
    pub url: Option<String>,
    /// Horodatage metier expose aux clients.
    pub timestamp: String,
    /// Date de creation cote translation.
    pub created_at: String,
    /// Date de mise a jour cote translation.
    pub updated_at: String,
}
