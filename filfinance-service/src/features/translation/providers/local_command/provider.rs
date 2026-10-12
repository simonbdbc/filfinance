//! Provider local one-shot pilote par une commande externe.

// `execution` isole le process bloquant, `response` isole le mapping du JSON de sortie.
mod execution;
mod response;

use crate::{
    errors::AppError,
    features::translation::{
        domain::{ProviderTranslation, TranslationRequest},
        providers::heuristic::protect_finance_tokens,
    },
};

use super::dto::LocalCommandPayload;

/// Provider local pilote via une commande externe pour brancher OPUS-MT.
#[derive(Debug, Clone)]
pub(crate) struct LocalCommandTranslationProvider {
    provider_name: String,
    program: String,
    args: Vec<String>,
    timeout_ms: u64,
}

impl LocalCommandTranslationProvider {
    /// Construit un provider one-shot avec sa commande deja resolue depuis la config.
    pub(crate) fn new(
        provider_name: String,
        program: String,
        args: Vec<String>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            provider_name,
            program,
            args,
            timeout_ms,
        }
    }

    /// Retourne le nom provider pour les logs sans exposer les champs internes.
    pub(crate) fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Donne acces aux arguments effectifs uniquement aux tests d'integration module.
    #[cfg(test)]
    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    /// Appelle une commande locale compatible avec le contrat JSON du service.
    pub(crate) async fn translate(
        &self,
        request: &TranslationRequest,
    ) -> Result<ProviderTranslation, AppError> {
        // Les tokens finance sont masqués avant le passage au modèle externe, puis restaurés après.
        let protected_title = protect_finance_tokens(&request.source_title);
        let protected_body = request.source_body.as_deref().map(protect_finance_tokens);

        let payload = LocalCommandPayload {
            source_language: &request.source_language,
            target_language: &request.target_language,
            title: &protected_title.masked_text,
            body: protected_body
                .as_ref()
                .map(|body| body.masked_text.as_str()),
        };
        let payload_bytes = serde_json::to_vec(&payload)?;
        let response = execution::execute_local_command(
            self.program.clone(),
            self.args.clone(),
            payload_bytes,
            self.provider_name.clone(),
            self.timeout_ms,
        )
        .await?;

        response::provider_translation_from_response(
            response,
            &self.provider_name,
            &protected_title,
            protected_body.as_ref(),
        )
    }
}
