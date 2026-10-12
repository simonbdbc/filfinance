//! Decouverte du catalogue local de voix Piper.

// `config`, `entry` et `speaker` isolent lecture JSON, filtrage disque et expansion du catalogue.
mod config;
mod entry;
mod speaker;

use std::path::Path;

use crate::errors::AppError;

use super::types::PiperVoiceSelection;
use entry::discover_voice_directory_entry;

/// Retourne le catalogue local complet des selections FR, flattene par speaker si le modele le permet.
pub(in crate::features::audio) fn discover_piper_french_voice_selections(
    models_dir: &str,
) -> Result<Vec<PiperVoiceSelection>, AppError> {
    let root_path = Path::new(models_dir);
    if !root_path.exists() {
        return Err(AppError::downstream(format!(
            "tts models dir does not exist: {}",
            root_path.display()
        )));
    }

    let mut selections: Vec<PiperVoiceSelection> = Vec::new();
    let directory_entries = std::fs::read_dir(root_path).map_err(AppError::from)?;

    for entry in directory_entries {
        let entry = entry.map_err(AppError::from)?;
        if let Some(mut voice_selections) = discover_voice_directory_entry(entry) {
            // Chaque dossier valide peut produire une ou plusieurs sélections selon le nombre de speakers.
            selections.append(&mut voice_selections);
        }
    }

    selections.sort_by(|left, right| left.selection_id.cmp(&right.selection_id));
    selections.dedup_by(|left, right| left.selection_id == right.selection_id);

    if selections.is_empty() {
        return Err(AppError::downstream(format!(
            "no french piper voice found in {}",
            root_path.display()
        )));
    }

    Ok(selections)
}
