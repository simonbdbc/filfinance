//! Types de dialogue entre le service traduction et ses providers.

use super::draft::NewsTranslationDraft;

/// Requete normalisee envoyee au provider de traduction.
#[derive(Debug, Clone)]
pub(crate) struct TranslationRequest {
    /// Titre source possede car la requete traverse des appels async ou threads.
    pub(crate) source_title: String,
    /// Corps source optionnel deja borne par la configuration.
    pub(crate) source_body: Option<String>,
    /// Langue source attendue par le provider.
    pub(crate) source_language: String,
    /// Langue cible attendue par le provider.
    pub(crate) target_language: String,
}

/// Reponse normalisee retournee par un provider de traduction.
#[derive(Debug, Clone)]
pub(crate) struct ProviderTranslation {
    /// Titre traduit et normalise par le provider.
    pub(crate) translated_title: String,
    /// Corps traduit optionnel.
    pub(crate) translated_body: Option<String>,
    /// Mode de traduction effectif renvoye ou choisi par le provider.
    pub(crate) translation_mode: String,
}

/// Resultat intermediaire du worker avant persistence.
#[derive(Debug, Clone)]
pub(crate) struct TranslationDraftBuild {
    /// Brouillon pret a etre persiste dans `news_translations`.
    pub(crate) draft: NewsTranslationDraft,
    /// Detail optionnel quand un fallback heuristique a ete necessaire.
    pub(crate) fallback_detail: Option<String>,
    /// Nombre de remplacements glossaire observes pendant ce brouillon.
    pub(crate) glossary_hits: u64,
}
