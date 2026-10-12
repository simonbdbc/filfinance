//! Assemblage du snapshot runtime traduction.

use std::sync::atomic::Ordering;

use super::TranslationRuntimeState;
use crate::features::translation::runtime::{
    samples::{pending_count, percentile_duration_ms},
    snapshot::TranslationRuntimeSnapshot,
};

impl TranslationRuntimeState {
    /// Retourne une photo serialisable de l'etat runtime.
    pub fn snapshot(&self) -> TranslationRuntimeSnapshot {
        let latest = self
            .latest
            .lock()
            .expect("translation runtime mutex poisoned")
            .clone();
        // Les compteurs sont lus une seule fois pour garder un snapshot coherent dans la reponse.
        let received_total = self.received_total.load(Ordering::Relaxed);
        let translated_total = self.translated_total.load(Ordering::Relaxed);
        let skipped_total = self.skipped_total.load(Ordering::Relaxed);
        let failed_total = self.failed_total.load(Ordering::Relaxed);
        // `pending_total` est une approximation utile de la pression worker sans inspecter Tokio.
        let pending_total = pending_count(
            received_total,
            translated_total,
            skipped_total,
            failed_total,
        );

        TranslationRuntimeSnapshot {
            enabled: self.enabled,
            provider: self.provider.clone(),
            profile: self.profile.clone(),
            source_language: self.source_language.clone(),
            target_language: self.target_language.clone(),
            worker_count: self.worker_count,
            warmup_enabled: self.warmup_enabled,
            received_total,
            translated_total,
            skipped_total,
            fallback_total: self.fallback_total.load(Ordering::Relaxed),
            failed_total,
            pending_total,
            last_translation_duration_ms: latest.last_translation_duration_ms,
            last_translation_mode: latest.last_translation_mode,
            last_success_at: latest.last_success_at,
            last_error_at: latest.last_error_at,
            last_error: latest.last_error,
            warmup_completed_at: latest.warmup_completed_at,
            warmup_duration_ms: latest.warmup_duration_ms,
            translation_duration_sample_count: latest.translation_duration_samples_ms.len(),
            glossary_entries_loaded: self.glossary_entries_loaded.load(Ordering::Relaxed),
            glossary_hits_total: self.glossary_hits_total.load(Ordering::Relaxed),
            glossary_last_error: latest.glossary_last_error,
            translation_duration_p50_ms: percentile_duration_ms(
                &latest.translation_duration_samples_ms,
                50,
            ),
            translation_duration_p95_ms: percentile_duration_ms(
                &latest.translation_duration_samples_ms,
                95,
            ),
        }
    }
}
