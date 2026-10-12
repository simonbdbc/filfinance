//! Brouillons de traduction avant persistence durable.

/// Brouillon de traduction interne avant persistence durable.
#[derive(Debug, Clone)]
pub struct NewsTranslationDraft {
    /// Langue cible voulue pour le brouillon.
    pub language: String,
    /// Statut calcule par le worker.
    pub status: String,
    /// Titre FR produit.
    pub translated_title: String,
    /// Corps FR produit ou absent.
    pub translated_body: Option<String>,
    /// Narration FR prete a diffuser.
    pub narration_text: String,
    /// Mode/provider qui a produit le texte.
    pub translation_mode: String,
    /// Erreur optionnelle si le provider a degrade ou echoue.
    pub error_message: Option<String>,
}
