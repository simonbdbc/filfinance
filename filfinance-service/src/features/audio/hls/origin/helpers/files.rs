//! Helpers fichiers et parsers de noms de segments HLS.

// `Path` et `PathBuf` representent les fichiers HLS locaux servis ensuite par Axum.
use std::path::{Path, PathBuf};

// `fs` sert a lire les segments et lister les sorties `ffmpeg`.
use tokio::fs;

use crate::errors::AppError;

/// Liste les fichiers `.ts` produits par `ffmpeg`, dans l'ordre lexical du pattern numerote.
pub(in crate::features::audio::hls::origin) async fn collect_segment_files(
    work_dir: &Path,
) -> Result<Vec<PathBuf>, AppError> {
    let mut entries = fs::read_dir(work_dir).await?;
    let mut segment_files = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("ts") {
            segment_files.push(path);
        }
    }

    segment_files.sort();
    Ok(segment_files)
}

/// Parse un nom public strict comme `42.ts`.
pub(in crate::features::audio) fn parse_segment_sequence(
    segment_name: &str,
) -> Result<u64, AppError> {
    let raw_sequence = segment_name
        .strip_suffix(".ts")
        .ok_or_else(|| AppError::not_found("hls segment name must end with .ts"))?;

    if raw_sequence.is_empty() || !raw_sequence.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(AppError::not_found("hls segment sequence is invalid"));
    }

    raw_sequence
        .parse::<u64>()
        .map_err(|_| AppError::not_found("hls segment sequence is invalid"))
}

/// Valide un nom de segment clip strict comme `segment-000.ts`.
pub(in crate::features::audio) fn parse_clip_segment_name(
    segment_name: &str,
) -> Result<String, AppError> {
    let raw_sequence = segment_name
        .strip_prefix("segment-")
        .and_then(|value| value.strip_suffix(".ts"))
        .ok_or_else(|| AppError::not_found("hls clip segment name is invalid"))?;

    if raw_sequence.is_empty() || !raw_sequence.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(AppError::not_found("hls clip segment sequence is invalid"));
    }

    Ok(segment_name.to_owned())
}

/// Transforme un identifiant interne en composant de chemin sans caractere special.
pub(in crate::features::audio::hls::origin) fn sanitize_file_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "stream".to_owned()
    } else {
        sanitized
    }
}
