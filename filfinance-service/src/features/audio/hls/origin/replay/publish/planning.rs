//! Planification des noms publics pour les segments HLS replay.

use crate::{errors::AppError, features::audio::domain::AudioStartPayload};

use super::super::super::{
    HlsOriginState,
    types::{HlsSegment, PlannedSegment, RenderedSegment},
};

impl HlsOriginState {
    /// Réserve les noms publics définitifs des segments rendus.
    pub(super) fn plan_segments(
        &self,
        rendered_segments: Vec<RenderedSegment>,
        local_day: &str,
        start_payload: &AudioStartPayload,
    ) -> Result<Vec<PlannedSegment>, AppError> {
        // Le mutex est pris seulement pour incrémenter la séquence de manière atomique.
        let mut inner = self.inner.lock().expect("hls origin mutex poisoned");
        let mut planned_segments = Vec::with_capacity(rendered_segments.len());
        for rendered_segment in rendered_segments {
            let mut sequence = inner.next_sequence;
            let mut file_path = self
                .segments_dir
                .join(local_day)
                .join(format!("{sequence}.ts"));
            // Après un redémarrage, des fichiers du même jour peuvent déjà exister sur le volume Docker.
            while file_path.exists() {
                sequence += 1;
                file_path = self
                    .segments_dir
                    .join(local_day)
                    .join(format!("{sequence}.ts"));
            }
            inner.next_sequence = sequence + 1;
            planned_segments.push(PlannedSegment {
                temp_path: rendered_segment.temp_path,
                segment: HlsSegment {
                    sequence,
                    duration_secs: rendered_segment.duration_secs,
                    file_path,
                    local_day: local_day.to_owned(),
                    translation_id: Some(start_payload.translation_id),
                    news_event_id: Some(start_payload.news_event_id),
                    stream_id: Some(start_payload.stream_id.clone()),
                    timestamp: Some(start_payload.timestamp.clone()),
                },
            });
        }

        Ok(planned_segments)
    }
}
