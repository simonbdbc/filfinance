//! Configuration du worker TTS serveur.

// Le module standard sert ici pour conserver exactement le fallback historique de NEWS_TTS_MODELS_DIR.
use std::env;

// Les helpers d'environnement gardent le parsing NEWS_* uniforme dans tous les sous-modules.
use super::{
    TtsConfig,
    environment::{env_bool, env_csv, env_or, env_u32, env_u64, env_usize},
    tts_defaults::{default_tts_helper_script_path, default_tts_models_dir},
};

// Les erreurs de validation doivent remonter au bootstrap applicatif.
use crate::errors::AppError;

impl TtsConfig {
    /// Construit la configuration TTS depuis l'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        // Les valeurs par defaut Piper pointent vers les scripts/assets du workspace compile.
        let tts_command_args = {
            let parsed = env_csv("NEWS_TTS_COMMAND_ARGS");
            if parsed.is_empty() {
                vec![default_tts_helper_script_path()]
            } else {
                parsed
            }
        };
        let config = Self {
            enabled: env_bool("NEWS_TTS_ENABLED", false)?,
            provider: env_or("NEWS_TTS_PROVIDER", "piper_local_session"),
            // Le defaut pointe vers un identifiant public complet modele-speaker.
            voice: env_or("NEWS_TTS_VOICE", "fr_FR-upmc-medium-jessica"),
            local_command_program: env_or("NEWS_TTS_COMMAND", "python3"),
            local_command_args: tts_command_args,
            models_dir: env::var("NEWS_TTS_MODELS_DIR")
                .unwrap_or_else(|_| default_tts_models_dir()),
            worker_count: env_usize("NEWS_TTS_WORKERS", 1)?,
            rate_words_per_minute: env_u32("NEWS_TTS_RATE_WORDS_PER_MINUTE", 185)?,
            output_sample_rate_hz: env_u32("NEWS_TTS_OUTPUT_SAMPLE_RATE_HZ", 22_050)?,
            command_timeout_ms: env_u64("NEWS_TTS_COMMAND_TIMEOUT_MS", 12_000)?,
            warmup_enabled: env_bool("NEWS_TTS_WARMUP_ENABLED", true)?,
            warmup_text: env_or(
                "NEWS_TTS_WARMUP_TEXT",
                "Test audio. Les clips HLS audio sont operationnels.",
            ),
        };
        validate_tts_config(&config)?;
        Ok(config)
    }
}

/// Valide la coherence de la configuration TTS avant le demarrage.
fn validate_tts_config(config: &TtsConfig) -> Result<(), AppError> {
    // Si la TTS est desactivee, on n'impose pas la presence des autres champs.
    if !config.enabled {
        return Ok(());
    }

    let provider = config.provider.trim();

    if config.worker_count == 0 {
        return Err(AppError::Config(
            "NEWS_TTS_WORKERS must be greater than 0".to_owned(),
        ));
    }

    if provider.is_empty() {
        return Err(AppError::Config(
            "NEWS_TTS_PROVIDER cannot be empty".to_owned(),
        ));
    }

    match provider {
        "piper_local_session" => {
            // Une voix vide rendrait le provider inutilisable des la premiere synthese.
            if config.voice.trim().is_empty() {
                return Err(AppError::Config(
                    "NEWS_TTS_VOICE cannot be empty when NEWS_TTS_ENABLED=true".to_owned(),
                ));
            }

            // Le helper local reste indispensable pour piloter Piper depuis le worker Rust.
            if config.local_command_program.trim().is_empty() {
                return Err(AppError::Config(
                    "NEWS_TTS_COMMAND cannot be empty when NEWS_TTS_ENABLED=true".to_owned(),
                ));
            }

            // Le backend doit savoir ou scanner les voix Piper installees localement.
            if config.models_dir.trim().is_empty() {
                return Err(AppError::Config(
                    "NEWS_TTS_MODELS_DIR cannot be empty when NEWS_TTS_ENABLED=true".to_owned(),
                ));
            }

            // Meme logique pour le timeout de commande.
            if config.command_timeout_ms == 0 {
                return Err(AppError::Config(
                    "NEWS_TTS_COMMAND_TIMEOUT_MS must be greater than 0".to_owned(),
                ));
            }

            // Une frequence audio a zero n'a pas de sens pour le rendu final.
            if config.output_sample_rate_hz == 0 {
                return Err(AppError::Config(
                    "NEWS_TTS_OUTPUT_SAMPLE_RATE_HZ must be greater than 0".to_owned(),
                ));
            }

            Ok(())
        }
        // Le projet refuse les providers non explicitement supportes.
        other => Err(AppError::Config(format!(
            "unsupported NEWS_TTS_PROVIDER: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_tts_config() -> TtsConfig {
        TtsConfig {
            enabled: true,
            provider: "piper_local_session".to_owned(),
            voice: "fr_FR-upmc-medium-jessica".to_owned(),
            local_command_program: "python3".to_owned(),
            local_command_args: vec!["scripts/piper_local_session.py".to_owned()],
            models_dir: "data/models/piper".to_owned(),
            worker_count: 1,
            rate_words_per_minute: 185,
            output_sample_rate_hz: 22_050,
            command_timeout_ms: 12_000,
            warmup_enabled: true,
            warmup_text: "Test audio.".to_owned(),
        }
    }

    #[test]
    fn validates_piper_local_session() {
        let config = base_tts_config();

        validate_tts_config(&config).expect("piper config should be valid");
    }

    #[test]
    fn rejects_non_piper_provider() {
        let mut config = base_tts_config();
        config.provider = "unsupported".to_owned();

        assert!(validate_tts_config(&config).is_err());
    }
}
