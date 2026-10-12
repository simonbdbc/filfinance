//! Enregistrements des publications HLS abouties ou echouees.

// `Ordering` suffit en relaxed pour des compteurs diagnostics monotones.
use std::sync::atomic::Ordering;

use crate::{
    features::audio::{domain::AudioStartPayload, hls::runtime::push_duration_sample},
    shared::time::now_rfc3339,
};

use super::super::HlsOriginState;

impl HlsOriginState {
    /// Memorise une publication HLS aboutie pour l'API runtime et les percentiles.
    pub(in crate::features::audio::hls::origin) fn record_publish_success(
        &self,
        start_payload: &AudioStartPayload,
        total_bytes: u64,
        segment_count: u64,
        publish_duration_ms: u64,
        render_duration_ms: u64,
    ) {
        self.replay_published_total.fetch_add(1, Ordering::Relaxed);
        let mut latest = self.latest.lock().expect("hls runtime mutex poisoned");
        latest.last_publish_duration_ms = Some(publish_duration_ms);
        latest.last_render_duration_ms = Some(render_duration_ms);
        latest.last_segment_count = Some(segment_count);
        latest.last_stream_id = Some(start_payload.stream_id.clone());
        latest.last_total_bytes = Some(total_bytes);
        latest.last_success_at = Some(now_rfc3339());
        latest.last_error = None;
        latest.last_error_at = None;
        push_duration_sample(&mut latest.publish_duration_samples_ms, publish_duration_ms);
        push_duration_sample(&mut latest.render_duration_samples_ms, render_duration_ms);
    }

    /// Memorise une publication HLS echouee sans masquer l'erreur originale.
    pub(in crate::features::audio::hls::origin) fn record_publish_failure(
        &self,
        error: impl Into<String>,
    ) {
        self.failed_total.fetch_add(1, Ordering::Relaxed);
        let mut latest = self.latest.lock().expect("hls runtime mutex poisoned");
        latest.last_error = Some(error.into());
        latest.last_error_at = Some(now_rfc3339());
    }

    /// Compte explicitement les echecs imputables au process `ffmpeg`.
    pub(in crate::features::audio::hls::origin) fn record_ffmpeg_failure(&self) {
        self.ffmpeg_failed_total.fetch_add(1, Ordering::Relaxed);
    }
}
