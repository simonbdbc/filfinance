//! Projection des fichiers temporaires HLS en segments rendus.

use std::path::PathBuf;

use super::super::{helpers::estimate_segment_duration, types::RenderedSegment};

/// Ajoute une duree estimee a chaque segment temporaire produit par ffmpeg.
pub(super) fn map_rendered_segments(
    segment_files: Vec<PathBuf>,
    total_duration: Option<f64>,
    segment_target_duration_secs: u32,
) -> Vec<RenderedSegment> {
    segment_files
        .into_iter()
        .enumerate()
        .map(|(index, temp_path)| RenderedSegment {
            temp_path,
            duration_secs: estimate_segment_duration(
                total_duration,
                index,
                segment_target_duration_secs,
            ),
        })
        .collect()
}
