//! Ajout du corps audio et finalisation d'un clip HLS.

// `Instant` mesure les durées internes d'encodage et de publication.
use std::time::Instant;

use crate::{errors::AppError, features::audio::hls::playlist::clip_playlist_url};

use super::super::super::{HlsOriginState, helpers::now_unix_ms, types::HlsClipFinalizeMetrics};

impl HlsOriginState {
    /// Ajoute le corps traduit à un clip existant puis finalise la playlist avec `#EXT-X-ENDLIST`.
    pub async fn append_clip_body_and_finalize(
        &self,
        stream_id: &str,
        body_wav_bytes: &[u8],
    ) -> Result<HlsClipFinalizeMetrics, AppError> {
        crate::features::calendar_logs::operation(
            "append_clip_body_and_finalize",
            serde_json::json!({}),
            async {
                if !self.enabled {
                    return Ok(HlsClipFinalizeMetrics::disabled(stream_id.to_owned()));
                }

                let publish_started_at = Instant::now();
                if body_wav_bytes.is_empty() {
                    return Err(AppError::downstream(
                        "cannot append an empty body audio payload to HLS clip".to_owned(),
                    ));
                }

                // Le worker calendrier appelle toujours l'intro avant le corps. Cette
                // premiere etape rehydrate son clip reserve si besoin; le live reste
                // donc entierement sur l'index memoire ici.
                let existing = self
                    .inner
                    .lock()
                    .expect("hls origin mutex poisoned")
                    .clips
                    .get(stream_id)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::not_found(format!("hls clip not found: {stream_id}"))
                    })?;
                if existing.finalized {
                    return Ok(HlsClipFinalizeMetrics {
                        hls_enabled: true,
                        stream_id: stream_id.to_owned(),
                        playlist_url: clip_playlist_url(stream_id),
                        appended_segment_count: 0,
                        total_segment_count: existing.segments.len() as u64,
                        publish_duration_ms: publish_started_at.elapsed().as_millis() as u64,
                        render_duration_ms: 0,
                    });
                }
                let next_index = existing.segments.len();
                let needs_discontinuity = !existing.segments.is_empty();

                let render_started_at = Instant::now();
                let rendered_segments = self
                    .render_segments_with_target(
                        &format!("clip-{stream_id}-body"),
                        body_wav_bytes,
                        self.clip_segment_target_duration_secs,
                    )
                    .await?;
                let render_duration_ms = render_started_at.elapsed().as_millis() as u64;
                let appended_segment_count = rendered_segments.len() as u64;
                let body_segments = self
                    .move_rendered_clip_segments(
                        stream_id,
                        rendered_segments,
                        next_index,
                        needs_discontinuity,
                        existing.durable_recovery,
                    )
                    .await?;
                let mut finalized_clip = existing;
                finalized_clip.segments.extend(body_segments.clone());
                finalized_clip.finalized = true;
                finalized_clip.updated_at_ms = now_unix_ms();
                // Comme pour l'intro, l'etat final est durable avant le commit
                // memoire: une coupure juste ici est idempotente au prochain worker.
                if finalized_clip.durable_recovery {
                    self.persist_clip_state(&finalized_clip).await?;
                }
                let total_segment_count = self.commit_clip_body(stream_id, body_segments)?;

                Ok(HlsClipFinalizeMetrics {
                    hls_enabled: true,
                    stream_id: stream_id.to_owned(),
                    playlist_url: clip_playlist_url(stream_id),
                    appended_segment_count,
                    total_segment_count: total_segment_count as u64,
                    publish_duration_ms: publish_started_at.elapsed().as_millis() as u64,
                    render_duration_ms,
                })
            },
        )
        .await
    }
}
