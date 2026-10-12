//! Restauration d'une reponse du helper de traduction local en session.

use crate::{
    errors::AppError,
    features::translation::{
        domain::ProviderTranslation,
        providers::{heuristic::ProtectedText, local_command::LocalCommandResponse},
    },
    shared::text::normalize_whitespace,
};

/// Restaure les tokens proteges et normalise la reponse du helper session.
pub(super) fn restore_session_translation(
    response: LocalCommandResponse,
    protected_title: ProtectedText,
    protected_body: Option<ProtectedText>,
    provider_name: &str,
) -> Result<ProviderTranslation, AppError> {
    let translated_title_raw = response
        .translated_title
        .or(response.title)
        .map(|value| normalize_whitespace(&value))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::downstream(
                "translation session returned an empty translated_title".to_owned(),
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
            protected_body
                .as_ref()
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
