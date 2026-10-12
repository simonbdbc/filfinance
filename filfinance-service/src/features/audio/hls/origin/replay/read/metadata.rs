//! Metadata HTTP du replay HLS du jour.

use crate::shared::time::now_rfc3339;

use super::super::super::{
    HLS_REPLAY_TODAY_PLAYLIST_URL, HlsOriginState,
    helpers::{current_local_day, replay_segments_for_day},
    types::HlsReplayTodayMetadata,
};

impl HlsOriginState {
    /// Retourne la metadata HTTP du replay HLS du jour.
    pub fn replay_today_metadata(&self) -> HlsReplayTodayMetadata {
        // La metadata doit répondre même si HLS est désactivé pour guider le client.
        let local_day = current_local_day();
        // On lit l'index courant sans faire d'I/O disque.
        let replay_segments = {
            let inner = self.inner.lock().expect("hls origin mutex poisoned");
            replay_segments_for_day(&inner.segments, &local_day)
        };
        // Les bornes de séquence aident le debug sans exposer de structure interne.
        let first_sequence = replay_segments.first().map(|segment| segment.sequence);
        let latest_sequence = replay_segments.last().map(|segment| segment.sequence);

        HlsReplayTodayMetadata {
            enabled: self.enabled,
            available: self.enabled && !replay_segments.is_empty(),
            local_day,
            playlist_url: HLS_REPLAY_TODAY_PLAYLIST_URL.to_owned(),
            segment_count: replay_segments.len(),
            first_sequence,
            latest_sequence,
            updated_at: now_rfc3339(),
        }
    }
}
