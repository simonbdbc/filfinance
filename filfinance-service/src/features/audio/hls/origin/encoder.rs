//! Encodage ffmpeg de WAV en segments HLS.

mod command;
mod mapping;
mod workspace;

use crate::errors::AppError;

use super::{
    HlsOriginState,
    helpers::{collect_segment_files, estimate_wav_duration_secs},
    types::RenderedSegment,
};
use command::run_ffmpeg_segmenter;
use mapping::map_rendered_segments;
use workspace::prepare_encoder_workspace;

impl HlsOriginState {
    /// Encode le WAV source en segments MPEG-TS/AAC avec une duree cible explicite.
    pub(in crate::features::audio::hls::origin) async fn render_segments_with_target(
        &self,
        stream_id: &str,
        wav_bytes: &[u8],
        segment_target_duration_secs: u32,
    ) -> Result<Vec<RenderedSegment>, AppError> {
        let workspace = prepare_encoder_workspace(&self.segments_dir, stream_id, wav_bytes).await?;

        let output = match run_ffmpeg_segmenter(
            &self.ffmpeg_command,
            &workspace.input_path,
            &workspace.output_pattern,
            segment_target_duration_secs,
        )
        .await
        {
            Ok(output) => output,
            Err(error) => {
                self.record_ffmpeg_failure();
                workspace.cleanup().await;
                return Err(AppError::from(error));
            }
        };

        if !output.status.success() {
            self.record_ffmpeg_failure();
            let stderr = String::from_utf8_lossy(&output.stderr);
            workspace.cleanup().await;
            return Err(AppError::downstream(format!(
                "ffmpeg HLS segment rendering failed: {}",
                stderr.trim()
            )));
        }

        let segment_files = collect_segment_files(&workspace.work_dir).await?;
        if segment_files.is_empty() {
            self.record_ffmpeg_failure();
            workspace.cleanup().await;
            return Err(AppError::downstream(
                "ffmpeg did not produce any HLS segment".to_owned(),
            ));
        }

        let total_duration = estimate_wav_duration_secs(wav_bytes);
        Ok(map_rendered_segments(
            segment_files,
            total_duration,
            segment_target_duration_secs,
        ))
    }
}
