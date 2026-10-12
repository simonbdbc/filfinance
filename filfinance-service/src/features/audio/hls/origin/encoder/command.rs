//! Construction et lancement de la commande ffmpeg HLS.

use std::{path::Path, process::Output};

// `Command` lance `ffmpeg` sans ajouter de dependance d'encodage audio dans le binaire Rust.
use tokio::process::Command;

use super::super::HLS_AUDIO_BITRATE;

/// Lance ffmpeg pour convertir le WAV d'entree en segments MPEG-TS.
pub(super) async fn run_ffmpeg_segmenter(
    ffmpeg_command: &str,
    input_path: &Path,
    output_pattern: &Path,
    segment_target_duration_secs: u32,
) -> std::io::Result<Output> {
    let mut diagnostic = crate::features::calendar_logs::Operation::start(
        "ffmpeg.segmenter",
        serde_json::json!({"program":ffmpeg_command,"input_path":input_path,"output_pattern":output_pattern,"segment_target_duration_secs":segment_target_duration_secs}),
    );
    let result = async {
        Command::new(ffmpeg_command)
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-y")
            .arg("-i")
            .arg(input_path)
            .arg("-vn")
            .arg("-map")
            .arg("0:a:0")
            .arg("-ac")
            .arg("1")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg(HLS_AUDIO_BITRATE)
            .arg("-f")
            .arg("segment")
            .arg("-segment_time")
            .arg(segment_target_duration_secs.to_string())
            .arg("-segment_format")
            .arg("mpegts")
            .arg("-reset_timestamps")
            .arg("1")
            .arg(output_pattern)
            .output()
            .await
    }
    .await;
    if let Ok(output) = &result {
        crate::features::calendar_logs::bytes("ffmpeg.stdout", &output.stdout);
        crate::features::calendar_logs::bytes("ffmpeg.stderr", &output.stderr);
    }
    diagnostic.complete(&result.as_ref().map(|output| serde_json::json!({"exit_code":output.status.code(),"success":output.status.success()})));
    result
}
