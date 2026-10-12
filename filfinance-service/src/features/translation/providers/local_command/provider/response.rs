//! Mapping de la reponse JSON locale vers le contrat provider.

use crate::{
    errors::AppError,
    features::translation::{domain::ProviderTranslation, providers::heuristic::ProtectedText},
    shared::text::normalize_whitespace,
};

use super::super::dto::LocalCommandResponse;

/// Convertit la reponse de commande en traduction applicative.
pub(super) fn provider_translation_from_response(
    response: LocalCommandResponse,
    provider_name: &str,
    protected_title: &ProtectedText,
    protected_body: Option<&ProtectedText>,
) -> Result<ProviderTranslation, AppError> {
    // Les anciens scripts pouvaient renvoyer `title/body`; les champs `translated_*` restent prioritaires.
    let translated_title_raw = response
        .translated_title
        .or(response.title)
        .map(|value| normalize_whitespace(&value))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::downstream(
                "translation command returned an empty translated_title".to_owned(),
            )
        })?;

    let translated_body_raw = response
        .translated_body
        .or(response.body)
        .map(|value| normalize_whitespace(&value))
        .filter(|value| !value.is_empty());

    let translated_title = protected_title
        .restore_provider_output(&translated_title_raw)
        .map_err(AppError::downstream)?;
    let translated_body = translated_body_raw
        .map(|body| {
            // `Option<&ProtectedText>` evite de déplacer le body protégé pendant la restauration.
            protected_body
                .map(|protected| protected.restore_provider_output(&body))
                .unwrap_or_else(|| Ok(body))
        })
        .transpose()
        .map_err(AppError::downstream)?;

    Ok(ProviderTranslation {
        translated_title,
        translated_body,
        translation_mode: response
            .translation_mode
            .unwrap_or_else(|| provider_name.to_owned()),
    })
}
