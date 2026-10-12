//! Appel HTTP unitaire vers Telegram.

// reqwest::Client execute les appels HTTP, StatusCode classe les erreurs retriables.
use reqwest::{Client, StatusCode, header::RETRY_AFTER};
// json construit le payload JSON sans creer une struct dediee pour cet appel simple.
use serde_json::json;

use super::error::TelegramSendError;

/// Effectue un appel HTTP unique vers Telegram et classe l'erreur pour le retry.
pub(super) async fn send_to_chat_once(
    client: &Client,
    endpoint: &str,
    chat_id: &str,
    text: &str,
) -> Result<(), TelegramSendError> {
    // Chaque envoi HTTP est volontairement stateless: on reconstruit la requete a chaque tentative.
    let response = client
        .post(endpoint)
        .json(&json!({
            "chat_id": chat_id,
            "text": text,
            "disable_web_page_preview": true,
        }))
        .send()
        .await
        .map_err(|error| TelegramSendError::Retriable {
            message: format!("telegram network error for chat {chat_id}: {error}"),
            retry_after_ms: None,
        })?;
    let status = response.status();

    if status.is_success() {
        return Ok(());
    }

    // On lit le corps texte meme en erreur pour garder un message exploitable en logs/support.
    let retry_after_ms = retry_after_delay_ms(&response);
    let response_text = response.text().await.unwrap_or_default();
    let message =
        format!("telegram api error for chat {chat_id}: status={status}, body={response_text}");

    if is_retriable_status(status) {
        Err(TelegramSendError::Retriable {
            message,
            retry_after_ms,
        })
    } else {
        Err(TelegramSendError::Permanent(message))
    }
}

/// Retourne la temporisation recommandee par Telegram si le header `Retry-After` est present.
fn retry_after_delay_ms(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(RETRY_AFTER)
        // Le header est texte; on le convertit d'abord en `&str`, puis en entier.
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| seconds.saturating_mul(1_000))
}

/// Identifie les statuts pour lesquels un retry borne reste utile.
fn is_retriable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}
