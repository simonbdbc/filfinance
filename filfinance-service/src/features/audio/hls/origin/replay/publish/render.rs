//! Rendu ffmpeg des segments temporaires du replay HLS.

// `Instant` mesure uniquement la phase d'encodage audio.
use std::time::Instant;

use crate::errors::AppError;

use super::super::super::{HlsOriginState, types::RenderedSegment};

/// Resultat du rendu replay avant planification des noms publics.
pub(super) struct ReplayRenderResult {
    /// Segments temporaires produits par `ffmpeg`.
    pub(super) rendered_segments: Vec<RenderedSegment>,
    /// Duree de rendu mesuree en millisecondes.
    pub(super) render_duration_ms: u64,
    /// Nombre de segments rendu, conserve avant transfert du vecteur.
    pub(super) segment_count: u64,
}

/// Rend le WAV en segments temporaires et journalise l'echec sur l'etat HLS.
pub(super) async fn render_replay_segments(
    origin: &HlsOriginState,
    stream_id: &str,
    wav_bytes: &[u8],
) -> Result<ReplayRenderResult, AppError> {
    // Le rendu HLS est volontairement fait hors mutex: `ffmpeg` peut prendre du temps.
    let render_started_at = Instant::now();
    let rendered_segments = match origin
        .render_segments_with_target(
            stream_id,
            wav_bytes,
            origin.replay_segment_target_duration_secs,
        )
        .await
    {
        Ok(rendered_segments) => rendered_segments,
        Err(error) => {
            origin.record_publish_failure(error.to_string());
            return Err(error);
        }
    };
    let render_duration_ms = render_started_at.elapsed().as_millis() as u64;
    let segment_count = rendered_segments.len() as u64;

    Ok(ReplayRenderResult {
        rendered_segments,
        render_duration_ms,
        segment_count,
    })
}
