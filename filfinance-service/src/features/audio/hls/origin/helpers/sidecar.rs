//! Sidecars de metadata des segments replay HLS.

// `Path` permet de retrouver le JSON adjacent a un segment `.ts`.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
// `fs` async ecrit les sidecars sans bloquer le worker TTS.
use tokio::fs;

use crate::{errors::AppError, features::audio::hls::origin::types::HlsSegment};

/// Metadata persistante minimale pour reconstruire l'index replay apres redemarrage.
#[derive(Debug, Deserialize, Serialize)]
pub(in crate::features::audio::hls::origin) struct HlsSegmentSidecar {
    pub(in crate::features::audio::hls::origin) duration_secs: f64,
    pub(in crate::features::audio::hls::origin) local_day: String,
    pub(in crate::features::audio::hls::origin) translation_id: Option<i64>,
    pub(in crate::features::audio::hls::origin) news_event_id: Option<i64>,
    pub(in crate::features::audio::hls::origin) stream_id: Option<String>,
    pub(in crate::features::audio::hls::origin) timestamp: Option<String>,
}

/// Charge le sidecar adjacent au segment si le fichier existe et reste lisible.
pub(in crate::features::audio::hls::origin) fn load_segment_sidecar(
    segment_path: &Path,
) -> Option<HlsSegmentSidecar> {
    let sidecar_path = segment_sidecar_path(segment_path);
    let bytes = std::fs::read(sidecar_path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Persiste la metadata de segment a cote du `.ts` definitif.
pub(in crate::features::audio::hls::origin) async fn write_segment_sidecar(
    segment: &HlsSegment,
) -> Result<(), AppError> {
    let sidecar = HlsSegmentSidecar {
        duration_secs: segment.duration_secs,
        local_day: segment.local_day.clone(),
        translation_id: segment.translation_id,
        news_event_id: segment.news_event_id,
        stream_id: segment.stream_id.clone(),
        timestamp: segment.timestamp.clone(),
    };
    let bytes = serde_json::to_vec(&sidecar).map_err(|error| {
        AppError::downstream(format!("failed to serialize hls replay sidecar: {error}"))
    })?;

    fs::write(segment_sidecar_path(&segment.file_path), bytes).await?;
    Ok(())
}

fn segment_sidecar_path(segment_path: &Path) -> PathBuf {
    segment_path.with_extension("json")
}
