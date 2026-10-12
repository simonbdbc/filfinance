//! Construction du snapshot operateur de l'origin HLS.

// `Ordering` suffit en relaxed pour lire des compteurs diagnostics.
use std::sync::atomic::Ordering;

use crate::features::audio::hls::runtime::{HlsRuntimeSnapshot, percentile_duration_ms};

use super::super::{
    HlsOriginState,
    helpers::{current_local_day, replay_segments_for_day},
};

impl HlsOriginState {
    /// Retourne les metriques runtime HLS lisibles par les routes operateur.
    pub fn snapshot(&self) -> HlsRuntimeSnapshot {
        // Le snapshot distingue clairement les clips temps reel du replay explicite.
        let local_day = current_local_day();
        let (replay_segments, clip_count) = {
            let inner = self.inner.lock().expect("hls origin mutex poisoned");
            (
                replay_segments_for_day(&inner.segments, &local_day),
                inner.clips.len(),
            )
        };
        // Les sequences replay visibles documentent le bord courant sans exposer les chemins disque.
        let first_replay_sequence = replay_segments.first().map(|segment| segment.sequence);
        let latest_replay_sequence = replay_segments.last().map(|segment| segment.sequence);

        let latest = self
            .latest
            .lock()
            .expect("hls runtime mutex poisoned")
            .clone();

        HlsRuntimeSnapshot {
            enabled: self.enabled,
            ffmpeg_command: self.ffmpeg_command.clone(),
            replay_segment_target_duration_secs: self.replay_segment_target_duration_secs,
            clip_segment_target_duration_secs: self.clip_segment_target_duration_secs,
            clip_count,
            replay_segment_count: replay_segments.len(),
            first_replay_sequence,
            latest_replay_sequence,
            replay_published_total: self.replay_published_total.load(Ordering::Relaxed),
            clip_playlist_requests_total: self.clip_playlist_requests_total.load(Ordering::Relaxed),
            clip_segment_requests_total: self.clip_segment_requests_total.load(Ordering::Relaxed),
            replay_playlist_requests_total: self
                .replay_playlist_requests_total
                .load(Ordering::Relaxed),
            replay_segment_requests_total: self
                .replay_segment_requests_total
                .load(Ordering::Relaxed),
            failed_total: self.failed_total.load(Ordering::Relaxed),
            ffmpeg_failed_total: self.ffmpeg_failed_total.load(Ordering::Relaxed),
            last_publish_duration_ms: latest.last_publish_duration_ms,
            last_render_duration_ms: latest.last_render_duration_ms,
            last_segment_count: latest.last_segment_count,
            last_stream_id: latest.last_stream_id,
            last_total_bytes: latest.last_total_bytes,
            last_success_at: latest.last_success_at,
            last_error_at: latest.last_error_at,
            last_error: latest.last_error,
            publish_duration_sample_count: latest.publish_duration_samples_ms.len(),
            publish_duration_p50_ms: percentile_duration_ms(
                &latest.publish_duration_samples_ms,
                50,
            ),
            publish_duration_p95_ms: percentile_duration_ms(
                &latest.publish_duration_samples_ms,
                95,
            ),
            render_duration_sample_count: latest.render_duration_samples_ms.len(),
            render_duration_p50_ms: percentile_duration_ms(&latest.render_duration_samples_ms, 50),
            render_duration_p95_ms: percentile_duration_ms(&latest.render_duration_samples_ms, 95),
        }
    }
}
