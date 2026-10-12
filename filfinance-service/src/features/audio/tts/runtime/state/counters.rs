//! Compteurs atomiques simples du runtime TTS.

use std::sync::atomic::Ordering;

use super::TtsRuntimeState;

impl TtsRuntimeState {
    /// Marque qu'une traduction FR est entree dans le worker TTS.
    pub fn record_received(&self) {
        self.received_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Marque qu'un item a ete ignore cote TTS.
    pub fn record_skipped(&self) {
        self.skipped_total.fetch_add(1, Ordering::Relaxed);
    }
}
