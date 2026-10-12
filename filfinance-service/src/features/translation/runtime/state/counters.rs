//! Compteurs atomiques du runtime traduction.

use std::sync::atomic::Ordering;

use super::TranslationRuntimeState;

impl TranslationRuntimeState {
    /// Incremente le compteur d'items recus par le worker.
    pub fn record_received(&self) {
        self.received_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Incremente le compteur d'items ignores car non eligibles.
    pub fn record_skipped(&self) {
        self.skipped_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Incremente le compteur de fallback heuristique.
    pub fn record_fallback(&self) {
        self.fallback_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Memorise le nombre courant d'entrees glossaire actives chargeables.
    pub fn record_glossary_loaded(&self, entries_loaded: u64) {
        self.glossary_entries_loaded
            .store(entries_loaded, Ordering::Relaxed);
    }

    /// Ajoute les remplacements glossaire observes dans le pipeline recent.
    pub fn record_glossary_hits(&self, hits: u64) {
        if hits > 0 {
            self.glossary_hits_total.fetch_add(hits, Ordering::Relaxed);
        }
    }
}
