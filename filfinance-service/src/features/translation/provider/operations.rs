//! Operations de traduction communes aux providers concrets.

use crate::{
    errors::AppError,
    features::translation::{
        domain::{ProviderTranslation, TranslationRequest},
        provider::{HEURISTIC_PROVIDER, TranslationProvider},
    },
};

impl TranslationProvider {
    /// Retourne le nom du provider pour les logs et les traces.
    pub(crate) fn provider_name(&self) -> &str {
        match self {
            Self::Heuristic(_) => HEURISTIC_PROVIDER,
            Self::LocalCommand(provider) => provider.provider_name(),
            Self::LocalCommandSession(provider) => provider.provider_name(),
        }
    }

    /// Traduit un item via le provider configure.
    pub(crate) async fn translate(
        &mut self,
        request: &TranslationRequest,
    ) -> Result<ProviderTranslation, AppError> {
        // L'appelant ne connait pas l'implementation concrete; l'enum assure le dispatch.
        match self {
            Self::Heuristic(provider) => Ok(provider.translate(request)),
            Self::LocalCommand(provider) => provider.translate(request).await,
            Self::LocalCommandSession(provider) => provider.translate(request).await,
        }
    }
}
