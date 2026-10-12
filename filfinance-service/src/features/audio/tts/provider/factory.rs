//! Construction du provider TTS concret depuis la configuration.

use std::sync::Arc;

use crate::{
    config::TtsConfig,
    features::audio::tts::{
        piper_session::{PIPER_LOCAL_SESSION_PROVIDER, PiperLocalSessionProvider},
        provider::kind::TtsProvider,
        runtime::TtsRuntimeState,
    },
};

impl TtsProvider {
    /// Construit le provider concret a partir de la configuration chargee au demarrage.
    pub(in crate::features::audio) fn from_config(
        config: &TtsConfig,
        runtime_state: Arc<TtsRuntimeState>,
    ) -> Self {
        match config.provider.as_str() {
            PIPER_LOCAL_SESSION_PROVIDER => build_piper_local_session(config, runtime_state),
            _ => {
                // La validation interdit deja les autres providers; ce fallback garde un chemin
                // exploitable si un test construit une config incomplete a la main.
                build_piper_local_session_with_name(
                    config,
                    runtime_state,
                    PIPER_LOCAL_SESSION_PROVIDER.to_owned(),
                )
            }
        }
    }
}

/// Construit le provider Piper avec le nom configure.
fn build_piper_local_session(
    config: &TtsConfig,
    runtime_state: Arc<TtsRuntimeState>,
) -> TtsProvider {
    build_piper_local_session_with_name(config, runtime_state, config.provider.clone())
}

/// Construit le provider Piper avec un nom explicite pour le fallback valide.
fn build_piper_local_session_with_name(
    config: &TtsConfig,
    runtime_state: Arc<TtsRuntimeState>,
    provider_name: String,
) -> TtsProvider {
    TtsProvider::PiperLocalSession(PiperLocalSessionProvider::new(
        runtime_state,
        provider_name,
        config.local_command_program.clone(),
        config.local_command_args.clone(),
        config.models_dir.clone(),
        config.rate_words_per_minute,
        config.command_timeout_ms,
    ))
}
