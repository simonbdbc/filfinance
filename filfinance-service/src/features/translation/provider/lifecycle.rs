//! Cycle de vie des providers de traduction.

use tracing::info;

use crate::{
    config::TranslationConfig,
    errors::AppError,
    features::translation::{domain::TranslationRequest, provider::TranslationProvider},
};

impl TranslationProvider {
    /// Permet de charger a froid le provider avant les premieres vraies news.
    pub(crate) async fn warmup(&mut self, config: &TranslationConfig) -> Result<(), AppError> {
        if !config.warmup_enabled {
            return Ok(());
        }

        // On passe par la meme interface que pour une vraie news afin de tester tout le wiring provider.
        let request = TranslationRequest {
            source_title: config.warmup_source_text.clone(),
            source_body: None,
            source_language: config.source_language.clone(),
            target_language: config.target_language.clone(),
        };

        match self {
            Self::Heuristic(provider) => {
                let _ = provider.translate(&request);
                info!("heuristic translation provider warmup completed");
                Ok(())
            }
            Self::LocalCommand(provider) => {
                provider.translate(&request).await?;
                info!(
                    provider = provider.provider_name(),
                    "local translation provider warmup completed"
                );
                Ok(())
            }
            Self::LocalCommandSession(provider) => {
                provider.translate(&request).await?;
                info!(
                    provider = provider.provider_name(),
                    "local translation session provider warmup completed"
                );
                Ok(())
            }
        }
    }

    /// Libere explicitement les ressources du provider avant l'arret du process.
    pub(crate) async fn shutdown(&mut self) {
        match self {
            Self::Heuristic(_) | Self::LocalCommand(_) => {}
            Self::LocalCommandSession(provider) => provider.shutdown().await,
        }
    }
}
