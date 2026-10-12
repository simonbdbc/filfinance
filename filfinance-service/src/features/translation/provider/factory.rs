//! Construction du provider concret depuis la configuration.

use crate::{
    config::TranslationConfig,
    features::translation::{
        provider::{
            HEURISTIC_PROVIDER, LOCAL_COMMAND_PROVIDER, LOCAL_COMMAND_SESSION_PROVIDER,
            OPUS_MT_LOCAL_COMMAND_PROVIDER, OPUS_MT_LOCAL_SESSION_PROVIDER, TranslationProvider,
        },
        providers::{
            heuristic::HeuristicTranslationProvider,
            local_command::{LocalCommandTranslationProvider, resolved_local_command_args},
            local_command_session::LocalCommandSessionTranslationProvider,
        },
    },
};

impl TranslationProvider {
    /// Construit le provider concret a partir de la configuration chargee au demarrage.
    pub(crate) fn from_config(config: &TranslationConfig) -> Self {
        match config.provider.as_str() {
            HEURISTIC_PROVIDER => Self::Heuristic(HeuristicTranslationProvider),
            LOCAL_COMMAND_PROVIDER | OPUS_MT_LOCAL_COMMAND_PROVIDER => {
                let args = resolved_local_command_args(config);
                // Le mode one-shot relance un process par traduction: simple, mais plus couteux.
                Self::LocalCommand(LocalCommandTranslationProvider::new(
                    config.provider.clone(),
                    config.local_command_program.clone().unwrap_or_default(),
                    args,
                    config.local_command_timeout_ms,
                ))
            }
            LOCAL_COMMAND_SESSION_PROVIDER | OPUS_MT_LOCAL_SESSION_PROVIDER => {
                let args = resolved_local_command_args(config);
                // Le mode session garde le helper charge en memoire et parle en JSONL sur stdin/stdout.
                Self::LocalCommandSession(Box::new(LocalCommandSessionTranslationProvider::new(
                    config.provider.clone(),
                    config.local_command_program.clone().unwrap_or_default(),
                    args,
                    config.local_command_timeout_ms,
                )))
            }
            // La validation de config interdit deja ces cas. Le fallback heuristique
            // protege toutefois le worker si le code est appele avec une config incoherente.
            _ => Self::Heuristic(HeuristicTranslationProvider),
        }
    }
}
