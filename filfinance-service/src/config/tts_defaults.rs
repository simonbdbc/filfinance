//! Chemins par defaut des assets et helpers TTS locaux.

/// Retourne le chemin du helper session Piper embarque dans le repo.
pub(super) fn default_tts_helper_script_path() -> String {
    format!(
        "{}/scripts/piper_local_session.py",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Retourne le repertoire par defaut des voix Piper installees localement.
pub(super) fn default_tts_models_dir() -> String {
    format!("{}/data/models/piper", env!("CARGO_MANIFEST_DIR"))
}
