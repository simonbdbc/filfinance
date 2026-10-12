//! Publication initiale de l'intro d'un clip HLS progressif.

// `Instant` mesure les durées internes d'encodage et de publication.
use std::time::Instant;

use crate::{
    errors::AppError,
    features::audio::{domain::AudioStartPayload, hls::playlist::clip_playlist_url},
};

use super::super::super::{
    HlsOriginState,
    helpers::now_unix_ms,
    types::{HlsClip, HlsClipInitialPublish},
};

impl HlsOriginState {
    /// Publie l'intro d'un clip HLS progressif et rend la playlist initiale lisible.
    pub async fn publish_clip_intro(
        &self,
        start_payload: &AudioStartPayload,
        intro_wav_bytes: &[u8],
    ) -> Result<HlsClipInitialPublish, AppError> {
        crate::features::calendar_logs::operation(
            "publish_clip_intro",
            serde_json::json!({}),
            async {
                if !self.enabled {
                    return Ok(HlsClipInitialPublish::disabled(
                        start_payload.stream_id.clone(),
                    ));
                }

                let publish_started_at = Instant::now();
                if intro_wav_bytes.is_empty() {
                    return Err(AppError::downstream(
                        "cannot publish an empty intro audio payload to HLS clip".to_owned(),
                    ));
                }

                let durable_recovery = start_payload.source == "calendar_day";

                // Les travaux calendrier reservent leur stream avant Piper. Si un
                // crash a deja rendu le clip (ou seulement son intro), l'etat HLS
                // durable suffit a reprendre exactement ce stream sans reencoder ni
                // risquer un second clip.
                if durable_recovery {
                    if let Some(existing) = self
                        .restore_clip_if_persisted(&start_payload.stream_id)
                        .await?
                    {
                        return Ok(HlsClipInitialPublish {
                            hls_enabled: true,
                            stream_id: start_payload.stream_id.clone(),
                            playlist_url: clip_playlist_url(&start_payload.stream_id),
                            segment_count: existing.segments.len() as u64,
                            publish_duration_ms: publish_started_at.elapsed().as_millis() as u64,
                            render_duration_ms: 0,
                        });
                    }
                }

                let render_started_at = Instant::now();
                let rendered_segments = self
                    .render_segments_with_target(
                        &format!("clip-{}-intro", start_payload.stream_id),
                        intro_wav_bytes,
                        self.clip_segment_target_duration_secs,
                    )
                    .await?;
                let render_duration_ms = render_started_at.elapsed().as_millis() as u64;
                let segment_count = rendered_segments.len() as u64;
                let clip_segments = self
                    .move_rendered_clip_segments(
                        &start_payload.stream_id,
                        rendered_segments,
                        0,
                        false,
                        durable_recovery,
                    )
                    .await?;
                let clip = HlsClip {
                    stream_id: start_payload.stream_id.clone(),
                    target_duration_secs: self.clip_segment_target_duration_secs,
                    segments: clip_segments.clone(),
                    finalized: false,
                    durable_recovery,
                    updated_at_ms: now_unix_ms(),
                };
                // Le commit de cet etat precede volontairement l'index memoire. Ainsi
                // une interruption entre l'ecriture des `.ts` et la publication
                // visible se repare au prochain claim fence de la file calendrier.
                if durable_recovery {
                    self.persist_clip_state(&clip).await?;
                }
                self.commit_clip_intro(&start_payload.stream_id, clip_segments, durable_recovery)?;

                Ok(HlsClipInitialPublish {
                    hls_enabled: true,
                    stream_id: start_payload.stream_id.clone(),
                    playlist_url: clip_playlist_url(&start_payload.stream_id),
                    segment_count,
                    publish_duration_ms: publish_started_at.elapsed().as_millis() as u64,
                    render_duration_ms,
                })
            },
        )
        .await
    }
}
