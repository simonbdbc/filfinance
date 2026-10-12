//! Resolution et canonisation des voix Piper.

use crate::{config::TtsConfig, errors::AppError};

use super::super::PIPER_LOCAL_SESSION_PROVIDER;
use super::discovery::discover_piper_french_voice_selections;

/// Canonise la voix configuree pour retomber sur un identifiant public stable du catalogue local.
pub(in crate::features::audio::tts) fn resolve_initial_tts_voice(config: &TtsConfig) -> String {
    if config.provider.trim() != PIPER_LOCAL_SESSION_PROVIDER {
        return config.voice.clone();
    }

    canonicalize_piper_voice_selection(&config.models_dir, &config.voice)
        .unwrap_or_else(|_| config.voice.clone())
}

/// Convertit une ancienne valeur `NEWS_TTS_VOICE` ou un choix runtime en identifiant public canonique.
pub(in crate::features::audio) fn canonicalize_piper_voice_selection(
    models_dir: &str,
    requested_voice: &str,
) -> Result<String, AppError> {
    let requested_voice = requested_voice.trim();
    if requested_voice.is_empty() {
        return Err(AppError::downstream(
            "requested piper voice cannot be empty".to_owned(),
        ));
    }

    let selections = discover_piper_french_voice_selections(models_dir)?;

    // Si l'appelant demande deja un identifiant public complet, on le garde tel quel.
    if let Some(selection) = selections
        .iter()
        .find(|selection| selection.selection_id == requested_voice)
    {
        return Ok(selection.selection_id.clone());
    }

    // Si l'appelant a encore une ancienne valeur "modele nu", on le mappe vers la premiere option.
    if let Some(selection) = selections
        .iter()
        .find(|selection| selection.voice_id == requested_voice)
    {
        return Ok(selection.selection_id.clone());
    }

    Err(AppError::downstream(format!(
        "requested piper voice is unknown in {}: {}",
        models_dir, requested_voice
    )))
}
