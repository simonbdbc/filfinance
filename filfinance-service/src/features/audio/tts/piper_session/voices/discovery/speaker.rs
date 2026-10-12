//! Expansion mono- ou multi-speaker d'une voix Piper.

use super::config::PiperVoiceConfigFile;
use crate::features::audio::tts::piper_session::voices::types::PiperVoiceSelection;

/// Transforme une configuration Piper en sélections plates utilisables par l'UI.
pub(super) fn voice_selections_from_config(
    voice_id: String,
    voice_config: PiperVoiceConfigFile,
) -> Vec<PiperVoiceSelection> {
    let mut ordered_speakers = voice_config
        .speaker_id_map
        .into_iter()
        .collect::<Vec<(String, u32)>>();
    // L'ordre par `speaker_id` reproduit l'ordre interne du modèle Piper.
    ordered_speakers.sort_by(|(left_name, left_id), (right_name, right_id)| {
        left_id
            .cmp(right_id)
            .then_with(|| left_name.cmp(right_name))
    });

    if voice_config.num_speakers.unwrap_or(1) > 1 && !ordered_speakers.is_empty() {
        return ordered_speakers
            .into_iter()
            .map(|(speaker_name, _speaker_id)| {
                // Un modèle multi-speaker est flattené en plusieurs options plates du sélecteur.
                PiperVoiceSelection {
                    selection_id: format!("{voice_id}-{speaker_name}"),
                    voice_id: voice_id.clone(),
                }
            })
            .collect();
    }

    // Les modèles mono-speaker gardent leur identifiant historique pour ne pas polluer l'UI.
    vec![PiperVoiceSelection {
        selection_id: voice_id.clone(),
        voice_id,
    }]
}
