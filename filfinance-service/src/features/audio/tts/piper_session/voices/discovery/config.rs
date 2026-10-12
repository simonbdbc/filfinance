//! Lecture du JSON de configuration d'une voix Piper.

use std::{collections::HashMap, path::Path};

use serde::Deserialize;
use tracing::warn;

/// Sous-ensemble du JSON `model.onnx.json` utile pour exposer un catalogue multi-speaker.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct PiperVoiceConfigFile {
    /// `num_speakers` indique si le modèle porte plusieurs locuteurs distincts.
    pub(super) num_speakers: Option<u32>,
    /// `speaker_id_map` relie un nom humain (`jessica`) à l'identifiant entier attendu par Piper.
    #[serde(default)]
    pub(super) speaker_id_map: HashMap<String, u32>,
}

/// Lit et parse un fichier `model.onnx.json`, en ignorant la voix si le fichier est invalide.
pub(super) fn read_voice_config(
    voice_id: &str,
    config_path: &Path,
) -> Option<PiperVoiceConfigFile> {
    // Le JSON local de Piper dit si le modèle est mono- ou multi-speaker, et donne les noms publics.
    let config_bytes = match std::fs::read(config_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            warn!(
                %error,
                voice_id,
                config_path = %config_path.display(),
                "failed to read piper voice config during discovery"
            );
            return None;
        }
    };

    match serde_json::from_slice::<PiperVoiceConfigFile>(&config_bytes) {
        Ok(config) => Some(config),
        Err(error) => {
            warn!(
                %error,
                voice_id,
                config_path = %config_path.display(),
                "failed to parse piper voice config during discovery"
            );
            None
        }
    }
}
