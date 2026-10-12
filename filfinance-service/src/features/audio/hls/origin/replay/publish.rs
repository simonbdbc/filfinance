//! Publication des segments HLS replay.

mod commit;
mod files;
mod planning;
mod render;

// `Instant` mesure les durées internes d'encodage et de publication.
use std::time::Instant;

// `info` trace les segments produits.
use tracing::info;

use crate::{errors::AppError, features::audio::domain::AudioStartPayload};

use super::super::{
    HlsOriginState,
    helpers::{current_local_day, local_day_for_rfc3339},
    types::HlsReplayPublishMetrics,
};
use render::render_replay_segments;

impl HlsOriginState {
    /// Publie un WAV TTS dans le replay HLS explicite du jour.
    pub async fn publish_replay_stream(
        &self,
        start_payload: &AudioStartPayload,
        wav_bytes: &[u8],
    ) -> Result<HlsReplayPublishMetrics, AppError> {
        // Si HLS est désactivé, le worker TTS continue exactement comme avant.
        if !self.enabled {
            return Ok(HlsReplayPublishMetrics::disabled());
        }

        // La mesure globale couvre l'encodage, les déplacements disque et le commit replay.
        let publish_started_at = Instant::now();

        // Un stream vide ne doit jamais créer une playlist replay incohérente.
        if wav_bytes.is_empty() {
            let error = AppError::downstream(
                "cannot publish an empty audio payload to HLS replay".to_owned(),
            );
            self.record_publish_failure(error.to_string());
            return Err(error);
        }

        // Le replay "du jour" est indexe sur le jour metier de la news, pas sur l'heure de generation TTS.
        let current_day = current_local_day();
        let payload_day =
            local_day_for_rfc3339(&start_payload.timestamp).unwrap_or_else(|| current_day.clone());
        if payload_day != current_day {
            info!(
                stream_id = %start_payload.stream_id,
                translation_id = start_payload.translation_id,
                news_event_id = start_payload.news_event_id,
                payload_day = %payload_day,
                current_day = %current_day,
                "hls replay stream skipped outside current product day"
            );
            return Ok(HlsReplayPublishMetrics::skipped_current_day());
        }

        let render_result =
            render_replay_segments(self, start_payload.stream_id.as_str(), wav_bytes).await?;
        // On conserve le nombre avant de transférer le vecteur vers la planification.
        let segment_count = render_result.segment_count;
        let local_day = payload_day;
        // Les séquences publiques sont réservées après encodage, juste avant le déplacement final.
        let planned_segments =
            match self.plan_segments(render_result.rendered_segments, &local_day, start_payload) {
                Ok(planned_segments) => planned_segments,
                Err(error) => {
                    self.record_publish_failure(error.to_string());
                    return Err(error);
                }
            };
        // Les fichiers doivent exister à leur URL canonique avant d'apparaître dans la playlist.
        if let Err(error) = self.move_planned_segments(&planned_segments).await {
            self.record_publish_failure(error.to_string());
            return Err(error);
        }
        // La playlist replay est mise à jour seulement après succès complet des déplacements.
        self.commit_segments(planned_segments, &local_day);
        // Les anciens jours sortent du contrat: leur suppression est best-effort pour ne jamais bloquer le replay.
        self.cleanup_expired_day_dirs(&local_day).await;
        let publish_duration_ms = publish_started_at.elapsed().as_millis() as u64;
        self.record_publish_success(
            start_payload,
            wav_bytes.len() as u64,
            segment_count,
            publish_duration_ms,
            render_result.render_duration_ms,
        );

        info!(
            stream_id = %start_payload.stream_id,
            translation_id = start_payload.translation_id,
            news_event_id = start_payload.news_event_id,
            publish_duration_ms = publish_duration_ms,
            render_duration_ms = render_result.render_duration_ms,
            segment_count = segment_count,
            "hls replay stream published"
        );

        Ok(HlsReplayPublishMetrics {
            hls_enabled: true,
            publish_duration_ms,
            render_duration_ms: render_result.render_duration_ms,
            segment_count,
        })
    }
}
