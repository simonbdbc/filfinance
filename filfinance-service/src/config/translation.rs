//! Configuration du worker de traduction texte.

// Les helpers d'environnement gardent le parsing NEWS_* uniforme dans tous les sous-modules.
use super::{
    TranslationConfig,
    environment::{env_bool, env_csv, env_optional, env_or, env_u64, env_usize},
    translation_profile::{
        normalize_translation_profile, parse_translation_profile_env,
        translation_command_args_contain_beam_size,
    },
};

// Les erreurs de validation doivent remonter au bootstrap applicatif.
use crate::errors::AppError;

impl TranslationConfig {
    /// Construit la configuration de traduction depuis l'environnement.
    pub fn from_env() -> Result<Self, AppError> {
        // La configuration de traduction est calculee a part pour valider le provider local tot.
        let config = Self {
            enabled: env_bool("NEWS_TRANSLATION_ENABLED", true)?,
            source_language: env_or("NEWS_TRANSLATION_SOURCE_LANGUAGE", "en"),
            target_language: env_or("NEWS_TRANSLATION_TARGET_LANGUAGE", "fr"),
            provider: env_or("NEWS_TRANSLATION_PROVIDER", "heuristic_fr"),
            // Le profil par defaut quality preserve le beam-size=4 historique.
            profile: parse_translation_profile_env()?,
            worker_count: env_usize("NEWS_TRANSLATION_WORKERS", 1)?,
            local_command_program: env_optional("NEWS_TRANSLATION_COMMAND"),
            local_command_args: env_csv("NEWS_TRANSLATION_COMMAND_ARGS"),
            local_command_timeout_ms: env_u64("NEWS_TRANSLATION_COMMAND_TIMEOUT_MS", 5_000)?,
            warmup_enabled: env_bool("NEWS_TRANSLATION_WARMUP_ENABLED", true)?,
            warmup_source_text: env_or(
                "NEWS_TRANSLATION_WARMUP_SOURCE_TEXT",
                "Federal Reserve signals rates stay higher for longer",
            ),
            max_body_chars: env_usize("NEWS_TRANSLATION_MAX_BODY_CHARS", 280)?,
        };
        validate_translation_config(&config)?;
        Ok(config)
    }
}

/// Valide la coherence de la configuration de traduction avant le demarrage.
fn validate_translation_config(config: &TranslationConfig) -> Result<(), AppError> {
    // `trim()` evite qu'une variable remplie d'espaces passe a tort la validation.
    let provider = config.provider.trim();

    if provider.is_empty() {
        return Err(AppError::Config(
            "NEWS_TRANSLATION_PROVIDER cannot be empty".to_owned(),
        ));
    }

    if normalize_translation_profile(&config.profile).is_none() {
        return Err(AppError::Config(format!(
            "NEWS_TRANSLATION_PROFILE must be one of fast, balanced, quality; got {}",
            config.profile
        )));
    }

    if config.worker_count == 0 {
        return Err(AppError::Config(
            "NEWS_TRANSLATION_WORKERS must be greater than 0".to_owned(),
        ));
    }

    // Le `match` force une enumeration exhaustive des providers supportes aujourd'hui.
    match provider {
        // Le mode heuristique reste le fallback local sans dependance externe.
        "heuristic_fr" => Ok(()),
        // Les modes commande locale preparent l'integration OPUS-MT via un helper local,
        // soit en one-shot, soit via une session stdin/stdout long-vivante.
        "local_command"
        | "opus_mt_local_command"
        | "local_command_session"
        | "opus_mt_local_session" => {
            // Cette chaine montre bien la valeur de l'idiome Rust:
            // - `as_deref()` passe de `Option<String>` a `Option<&str>`
            // - `map(str::trim)` nettoie sans allouer
            // - `filter(...)` rejette la chaine vide
            // - `is_none()` exprime clairement "commande absente ou vide"
            if config
                .local_command_program
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                return Err(AppError::Config(format!(
                    "NEWS_TRANSLATION_COMMAND is required when NEWS_TRANSLATION_PROVIDER={provider}"
                )));
            }

            // Un timeout a zero serait presque toujours une erreur de configuration.
            if config.local_command_timeout_ms == 0 {
                return Err(AppError::Config(
                    "NEWS_TRANSLATION_COMMAND_TIMEOUT_MS must be greater than 0".to_owned(),
                ));
            }

            // Le profil est la seule source de verite pour la qualite OPUS-MT.
            if translation_command_args_contain_beam_size(&config.local_command_args) {
                return Err(AppError::Config(
                    "NEWS_TRANSLATION_COMMAND_ARGS must not contain --beam-size; use NEWS_TRANSLATION_PROFILE=fast|balanced|quality".to_owned(),
                ));
            }

            Ok(())
        }
        // Toute autre valeur est refusee explicitement pour eviter un comportement implicite.
        other => Err(AppError::Config(format!(
            "unsupported NEWS_TRANSLATION_PROVIDER: {other}"
        ))),
    }
}
