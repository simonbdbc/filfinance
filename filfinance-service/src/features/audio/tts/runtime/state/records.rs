//! Enregistrements des derniers resultats TTS observables.

use std::sync::atomic::Ordering;

use crate::shared::time::now_rfc3339;

use super::{super::metrics::push_duration_sample, TtsRuntimeState};

impl TtsRuntimeState {
    /// Memorise un warmup TTS reussi.
    pub fn record_warmup_success(&self, duration_ms: u64) {
        let mut latest = self.latest.lock().expect("tts runtime mutex poisoned");
        latest.warmup_completed_at = Some(now_rfc3339());
        latest.warmup_duration_ms = Some(duration_ms);
    }

    /// Memorise une synthese TTS aboutie.
    pub fn record_generated(
        &self,
        stream_id: &str,
        duration_ms: u64,
        total_bytes: u64,
        chunk_count: u64,
    ) {
        self.generated_total.fetch_add(1, Ordering::Relaxed);
        let mut latest = self.latest.lock().expect("tts runtime mutex poisoned");
        latest.last_generation_duration_ms = Some(duration_ms);
        latest.last_total_bytes = Some(total_bytes);
        latest.last_chunk_count = Some(chunk_count);
        latest.last_stream_id = Some(stream_id.to_owned());
        latest.last_success_at = Some(now_rfc3339());
        latest.last_error = None;
        latest.last_error_at = None;
        push_duration_sample(&mut latest.generation_duration_samples_ms, duration_ms);
    }

    /// Memorise un echec TTS.
    pub fn record_failed(&self, error: impl Into<String>) {
        self.failed_total.fetch_add(1, Ordering::Relaxed);
        let mut latest = self.latest.lock().expect("tts runtime mutex poisoned");
        latest.last_error = Some(error.into());
        latest.last_error_at = Some(now_rfc3339());
    }
}
