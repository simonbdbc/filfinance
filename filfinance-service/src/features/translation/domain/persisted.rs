//! Traductions persistées par la feature news et diffusées aux downstreams.

// `Deserialize` et `Serialize` gardent le contrat JSON/SQLx des traductions persistées.
use serde::{Deserialize, Serialize};

/// Variante texte derivee d'un evenement de news pour un langage cible.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersistedNewsTranslation {
    /// Cle primaire SQLite de la traduction.
    pub id: i64,
    /// Cle etrangere vers `news_events`.
    pub news_event_id: i64,
    /// Langue cible de cette variante.
    pub language: String,
    /// Statut de traitement (`ready`, `error`, ...).
    pub status: String,
    /// Source de l'evenement d'origine, recopitee pour simplifier certains downstreams.
    pub source: String,
    /// Type d'evenement d'origine, aplati en `String`.
    pub event_kind: String,
    /// Titre source EN conserve pour debug et diffusion parallele.
    pub source_title: String,
    /// Corps source optionnel.
    pub source_body: Option<String>,
    /// Titre traduit en FR.
    pub translated_title: String,
    /// Corps traduit optionnel.
    pub translated_body: Option<String>,
    /// Texte de narration prepare pour Telegram et/ou TTS.
    pub narration_text: String,
    /// Mode de traduction effectivement utilise.
    pub translation_mode: String,
    /// Message d'erreur eventuel si la traduction a echoue.
    pub error_message: Option<String>,
    /// Date metier principale retenue pour les tris et replays.
    pub published_at: Option<String>,
    /// Date de creation de la ligne de traduction.
    pub created_at: String,
    /// Date de derniere mise a jour de la ligne.
    pub updated_at: String,
}
