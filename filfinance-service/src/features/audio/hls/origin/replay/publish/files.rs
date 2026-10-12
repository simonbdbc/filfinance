//! Déplacement et retention disque des segments replay.

// `fs` lit, déplace et nettoie les segments replay.
use tokio::fs;
// `warn` garde les erreurs non fatales de retention visibles.
use tracing::warn;

use crate::errors::AppError;

use super::super::super::{
    HlsOriginState,
    helpers::{looks_like_local_day, write_segment_sidecar},
    types::PlannedSegment,
};

impl HlsOriginState {
    /// Déplace les segments temporaires vers leurs chemins définitifs.
    pub(super) async fn move_planned_segments(
        &self,
        planned_segments: &[PlannedSegment],
    ) -> Result<(), AppError> {
        // Tous les segments d'un même publish sont dans le même dossier jour.
        if let Some(first_segment) = planned_segments.first()
            && let Some(parent_dir) = first_segment.segment.file_path.parent()
        {
            fs::create_dir_all(parent_dir).await?;
        }

        for planned_segment in planned_segments {
            fs::rename(
                &planned_segment.temp_path,
                &planned_segment.segment.file_path,
            )
            .await?;
            write_segment_sidecar(&planned_segment.segment).await?;
        }

        // Le dossier temporaire peut maintenant être supprimé; l'échec n'est pas fatal.
        if let Some(first_segment) = planned_segments.first()
            && let Some(tmp_dir) = first_segment.temp_path.parent()
        {
            let _ = fs::remove_dir_all(tmp_dir).await;
        }

        Ok(())
    }

    /// Supprime les dossiers de segments qui ne correspondent plus au jour local courant.
    pub(super) async fn cleanup_expired_day_dirs(&self, local_day: &str) {
        let Ok(mut entries) = fs::read_dir(&self.segments_dir).await else {
            return;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name == "tmp" || file_name == local_day || !looks_like_local_day(&file_name) {
                continue;
            }

            match entry.file_type().await {
                Ok(file_type) if file_type.is_dir() => {
                    if let Err(error) = fs::remove_dir_all(entry.path()).await {
                        warn!(%error, expired_day = %file_name, "failed to cleanup expired hls day directory");
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    warn!(%error, expired_day = %file_name, "failed to inspect hls day directory");
                }
            }
        }
    }
}
