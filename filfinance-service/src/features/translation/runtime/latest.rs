//! Donnees volatiles du runtime traduction protegees par mutex.

use crate::shared::time::now_rfc3339;

use super::samples::push_duration_sample;

/// Donnees volatiles mises a jour au fil de l'execution.
#[derive(Debug, Clone, Default)]
pub(super) struct TranslationRuntimeLatest {
    /// Derniere latence de traduction observee.
    pub(super) last_translation_duration_ms: Option<u64>,
    /// Mode renvoye par le provider pour la derniere traduction reussie.
    pub(super) last_translation_mode: Option<String>,
    /// Horodatage RFC3339 de la derniere traduction reussie.
    pub(super) last_success_at: Option<String>,
    /// Horodatage RFC3339 de la derniere erreur runtime.
    pub(super) last_error_at: Option<String>,
    /// Derniere erreur runtime utile au diagnostic.
    pub(super) last_error: Option<String>,
    /// Horodatage RFC3339 de fin de warmup.
    pub(super) warmup_completed_at: Option<String>,
    /// Duree du warmup provider en millisecondes.
    pub(super) warmup_duration_ms: Option<u64>,
    /// Derniere erreur de chargement ou application du glossaire.
    pub(super) glossary_last_error: Option<String>,
    /// Durees recentes de traductions reussies pour les percentiles.
    pub(super) translation_duration_samples_ms: Vec<u64>,
}

impl TranslationRuntimeLatest {
    /// Enregistre la derniere traduction aboutie et alimente la fenetre de latence.
    pub(super) fn record_success(&mut self, translation_mode: &str, duration_ms: u64) {
        self.last_translation_duration_ms = Some(duration_ms);
        self.last_translation_mode = Some(translation_mode.to_owned());
        self.last_success_at = Some(now_rfc3339());
        self.last_error = None;
        self.last_error_at = None;
        push_duration_sample(&mut self.translation_duration_samples_ms, duration_ms);
    }

    /// Memorise la derniere erreur utile au debug.
    pub(super) fn record_error(&mut self, error: impl Into<String>) {
        self.last_error = Some(error.into());
        self.last_error_at = Some(now_rfc3339());
    }

    /// Memorise la fin du warmup et sa latence.
    pub(super) fn record_warmup_success(&mut self, duration_ms: u64) {
        self.warmup_completed_at = Some(now_rfc3339());
        self.warmup_duration_ms = Some(duration_ms);
    }

    /// Memorise la derniere erreur glossaire sans marquer la traduction en echec.
    pub(super) fn record_glossary_error(&mut self, error: impl Into<String>) {
        self.glossary_last_error = Some(error.into());
    }

    /// Efface l'erreur glossaire apres un chargement valide.
    pub(super) fn clear_glossary_error(&mut self) {
        self.glossary_last_error = None;
    }
}
