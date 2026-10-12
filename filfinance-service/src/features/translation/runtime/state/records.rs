//! Mutations du dernier etat runtime traduction.

use std::sync::atomic::Ordering;

use super::TranslationRuntimeState;

impl TranslationRuntimeState {
    /// Enregistre une traduction aboutie avec sa latence et son mode.
    pub fn record_success(&self, translation_mode: &str, duration_ms: u64) {
        self.translated_total.fetch_add(1, Ordering::Relaxed);
        let mut latest = self
            .latest
            .lock()
            .expect("translation runtime mutex poisoned");
        latest.record_success(translation_mode, duration_ms);
    }

    /// Enregistre un echec persistant visible pour l'operateur.
    pub fn record_failure(&self, error: impl Into<String>) {
        self.failed_total.fetch_add(1, Ordering::Relaxed);
        self.record_error(error);
    }

    /// Memorise la derniere erreur utile au debug.
    pub fn record_error(&self, error: impl Into<String>) {
        let mut latest = self
            .latest
            .lock()
            .expect("translation runtime mutex poisoned");
        latest.record_error(error);
    }

    /// Memorise la fin du warmup et sa latence.
    pub fn record_warmup_success(&self, duration_ms: u64) {
        let mut latest = self
            .latest
            .lock()
            .expect("translation runtime mutex poisoned");
        latest.record_warmup_success(duration_ms);
    }

    /// Memorise une erreur non bloquante du glossaire.
    pub fn record_glossary_error(&self, error: impl Into<String>) {
        let mut latest = self
            .latest
            .lock()
            .expect("translation runtime mutex poisoned");
        latest.record_glossary_error(error);
    }

    /// Efface l'erreur glossaire quand le chargement redevient sain.
    pub fn clear_glossary_error(&self) {
        let mut latest = self
            .latest
            .lock()
            .expect("translation runtime mutex poisoned");
        latest.clear_glossary_error();
    }
}
