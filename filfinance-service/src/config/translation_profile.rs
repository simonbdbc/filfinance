//! Normalisation du profil de traduction et lecture de sa variable d'environnement.

// Le module standard permet de distinguer variable absente et valeur invalide.
use std::env;

// Les profils invalides doivent echouer pendant le bootstrap de configuration.
use crate::errors::AppError;

/// Retourne le profil de traduction canonique si la valeur est supportee.
pub(crate) fn normalize_translation_profile(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        // `fast` minimise la latence en gardant un seul chemin de decodage.
        "fast" => Some("fast"),
        // `balanced` est le candidat production a comparer avec l'observabilite NEWS-127.
        "balanced" => Some("balanced"),
        // `quality` reproduit le comportement historique `beam-size=4`.
        "quality" => Some("quality"),
        _ => None,
    }
}

/// Convertit un profil de traduction en beam-size OPUS-MT effectif.
pub(crate) fn translation_profile_beam_size(profile: &str) -> Option<u32> {
    match normalize_translation_profile(profile)? {
        "fast" => Some(1),
        "balanced" => Some(2),
        "quality" => Some(4),
        _ => None,
    }
}

/// Detecte un `--beam-size` configure manuellement dans les arguments de commande.
pub(crate) fn translation_command_args_contain_beam_size(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--beam-size" || arg.starts_with("--beam-size="))
}

/// Lit le profil OPUS-MT en preservant `quality` comme comportement historique.
pub(super) fn parse_translation_profile_env() -> Result<String, AppError> {
    match env::var("NEWS_TRANSLATION_PROFILE") {
        Ok(value) => normalize_translation_profile(&value)
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppError::Config(format!(
                    "NEWS_TRANSLATION_PROFILE must be one of fast, balanced, quality; got {}",
                    value.trim()
                ))
            }),
        // L'absence de variable garde beam-size=4 pour ne pas changer la prod existante.
        Err(env::VarError::NotPresent) => Ok("quality".to_owned()),
        Err(error) => Err(AppError::Config(format!(
            "failed to read NEWS_TRANSLATION_PROFILE: {error}"
        ))),
    }
}
