//! Filtrage d'une entrée de répertoire du catalogue Piper.

use std::fs::DirEntry;

use super::{config::read_voice_config, speaker::voice_selections_from_config};
use crate::features::audio::tts::piper_session::voices::types::PiperVoiceSelection;

/// Découvre une voix Piper depuis un dossier enfant de `models_dir`.
pub(super) fn discover_voice_directory_entry(entry: DirEntry) -> Option<Vec<PiperVoiceSelection>> {
    let Ok(file_type) = entry.file_type() else {
        return None;
    };
    if !file_type.is_dir() {
        return None;
    }

    let voice_id = entry.file_name().to_string_lossy().to_string();
    if !voice_id.starts_with("fr_") {
        return None;
    }

    let voice_dir = entry.path();
    let model_path = voice_dir.join("model.onnx");
    let config_path = voice_dir.join("model.onnx.json");
    if !model_path.exists() || !config_path.exists() {
        return None;
    }

    // Une entrée n'est exposée que si le modèle et son JSON sont présents et exploitables.
    let voice_config = read_voice_config(&voice_id, &config_path)?;
    Some(voice_selections_from_config(voice_id, voice_config))
}
