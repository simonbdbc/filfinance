//! Workspace temporaire de rendu HLS ffmpeg.

use std::path::{Path, PathBuf};

use tokio::fs;

use crate::errors::AppError;

use super::super::helpers::sanitize_file_component;

/// Chemins temporaires nécessaires a un rendu ffmpeg.
pub(super) struct EncoderWorkspace {
    /// Dossier de travail sous la racine HLS.
    pub(super) work_dir: PathBuf,
    /// WAV source ecrit pour ffmpeg.
    pub(super) input_path: PathBuf,
    /// Pattern de sortie des segments `segment_%05d.ts`.
    pub(super) output_pattern: PathBuf,
}

impl EncoderWorkspace {
    /// Nettoie le workspace temporaire de maniere best-effort.
    pub(super) async fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.work_dir).await;
    }
}

/// Prepare le dossier temporaire et ecrit le WAV source.
pub(super) async fn prepare_encoder_workspace(
    segments_dir: &Path,
    stream_id: &str,
    wav_bytes: &[u8],
) -> Result<EncoderWorkspace, AppError> {
    // Le stream id vient de notre worker, mais on le sanitise avant de l'utiliser en chemin.
    let safe_stream_id = sanitize_file_component(stream_id);
    // Le dossier temporaire reste sous la racine HLS pour que `rename` soit atomique.
    let work_dir = segments_dir.join("tmp").join(&safe_stream_id);
    // On nettoie un eventuel reliquat d'un ancien crash avant de produire de nouveaux fichiers.
    let _ = fs::remove_dir_all(&work_dir).await;
    // `create_dir_all` cree aussi la racine HLS si elle n'existe pas encore.
    fs::create_dir_all(&work_dir).await?;

    let input_path = work_dir.join("input.wav");
    let output_pattern = work_dir.join("segment_%05d.ts");
    fs::write(&input_path, wav_bytes).await?;

    Ok(EncoderWorkspace {
        work_dir,
        input_path,
        output_pattern,
    })
}
