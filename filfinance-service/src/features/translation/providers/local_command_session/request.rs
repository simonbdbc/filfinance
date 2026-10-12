//! Preparation d'une requete JSONL pour le provider local garde en session.

use crate::{
    errors::AppError,
    features::translation::{
        domain::TranslationRequest,
        providers::{
            heuristic::{ProtectedText, protect_finance_tokens},
            local_command::LocalCommandPayload,
        },
    },
};

/// Requete serialisee avec les tokens financiers proteges pour restauration apres traduction.
pub(super) struct PreparedSessionRequest {
    /// Ligne JSONL envoyee au helper long-vivant.
    pub(super) payload_line: String,
    /// Titre masque avant traduction.
    pub(super) protected_title: ProtectedText,
    /// Corps masque avant traduction quand il existe.
    pub(super) protected_body: Option<ProtectedText>,
}

/// Convertit une requete applicative en payload session borne au contrat local.
pub(super) fn prepare_session_request(
    request: &TranslationRequest,
) -> Result<PreparedSessionRequest, AppError> {
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

    Ok(PreparedSessionRequest {
        payload_line: format!("{}\n", serde_json::to_string(&payload)?),
        protected_title,
        protected_body,
    })
}
