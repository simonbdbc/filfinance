//! Construction du snapshot d'observabilite du runtime TTS.

use std::sync::atomic::Ordering;

use super::{
    super::{
        metrics::{pending_count, percentile_duration_ms},
        snapshot::TtsRuntimeSnapshot,
    },
    TtsRuntimeState,
};

impl TtsRuntimeState {
    /// Retourne un snapshot serialisable des metriques runtime TTS.
    pub fn snapshot(&self) -> TtsRuntimeSnapshot {
        let latest = self
            .latest
            .lock()
            .expect("tts runtime mutex poisoned")
            .clone();
        // Les compteurs sont lus une seule fois pour construire une photo lisible par l'operateur.
        let received_total = self.received_total.load(Ordering::Relaxed);
        let skipped_total = self.skipped_total.load(Ordering::Relaxed);
        let generated_total = self.generated_total.load(Ordering::Relaxed);
        let failed_total = self.failed_total.load(Ordering::Relaxed);
        // `pending_total` donne une approximation de la pression TTS sans exposer les details Tokio.
        let pending_total =
            pending_count(received_total, generated_total, skipped_total, failed_total);

        TtsRuntimeSnapshot {
            enabled: self.enabled,
            provider: self.provider.clone(),
            current_voice: self.current_voice(),
            warmup_enabled: self.warmup_enabled,
            worker_count: self.worker_count,
            output_sample_rate_hz: self.output_sample_rate_hz,
            received_total,
            skipped_total,
            generated_total,
            failed_total,
            pending_total,
            last_generation_duration_ms: latest.last_generation_duration_ms,
            last_total_bytes: latest.last_total_bytes,
            last_chunk_count: latest.last_chunk_count,
            last_stream_id: latest.last_stream_id,
            last_success_at: latest.last_success_at,
            last_error_at: latest.last_error_at,
            last_error: latest.last_error,
            warmup_completed_at: latest.warmup_completed_at,
            warmup_duration_ms: latest.warmup_duration_ms,
            voice_changed_at: latest.voice_changed_at,
            generation_duration_sample_count: latest.generation_duration_samples_ms.len(),
            generation_duration_p50_ms: percentile_duration_ms(
                &latest.generation_duration_samples_ms,
                50,
            ),
            generation_duration_p95_ms: percentile_duration_ms(
                &latest.generation_duration_samples_ms,
                95,
            ),
        }
    }
}
