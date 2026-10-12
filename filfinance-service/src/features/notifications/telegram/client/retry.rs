//! Politique de retry bornee pour Telegram.

// Future permet de typer une tentative de retry asynchrone injectable en test.
use std::future::Future;
// Duration garde les temporisations typees pour le rate limit et le backoff.
use std::time::Duration;

// reqwest::Client execute les tentatives HTTP reelles.
use reqwest::Client;
// Instant et sleep servent a la politique locale de rate limit et de retry.
use tokio::time::{Instant, sleep};
use tracing::warn;

use crate::{app_state::AppState, errors::AppError};

use super::{error::TelegramSendError, request::send_to_chat_once};

/// Applique le rate limit global du worker avant chaque appel HTTP sortant.
async fn enforce_rate_limit(state: &AppState, last_http_send_at: &mut Option<Instant>) {
    let rate_limit = Duration::from_millis(state.config.telegram.rate_limit_ms);

    if let Some(last_http_send_at_value) = last_http_send_at {
        // `elapsed()` compare le temps courant a l'instant stocke sans manipuler d'horloge systeme brute.
        let elapsed = last_http_send_at_value.elapsed();
        if elapsed < rate_limit {
            sleep(rate_limit - elapsed).await;
        }
    }
}

/// Execute un envoi Telegram avec un nombre borne de retries sur erreurs temporaires.
pub(in crate::features::notifications::telegram) async fn send_to_chat_with_retry(
    client: &Client,
    state: &AppState,
    endpoint: &str,
    chat_id: &str,
    text: &str,
    last_http_send_at: &mut Option<Instant>,
) -> Result<(), AppError> {
    execute_send_with_retry(state, chat_id, last_http_send_at, || {
        send_to_chat_once(client, endpoint, chat_id, text)
    })
    .await
}

/// Encapsule la politique de retry borne pour qu'elle reste testable sans HTTP reel.
pub(in crate::features::notifications) async fn execute_send_with_retry<TAttempt, TFut>(
    state: &AppState,
    chat_id: &str,
    last_http_send_at: &mut Option<Instant>,
    mut attempt_fn: TAttempt,
) -> Result<(), AppError>
where
    // `FnMut` autorise la closure a conserver un petit etat mutable entre tentatives.
    TAttempt: FnMut() -> TFut,
    TFut: Future<Output = Result<(), TelegramSendError>>,
{
    // `retry_attempts` compte seulement les retries; on ajoute 1 pour la premiere tentative.
    let total_attempts = state.config.telegram.retry_attempts + 1;

    for attempt_index in 0..total_attempts {
        enforce_rate_limit(state, last_http_send_at).await;

        match attempt_fn().await {
            Ok(()) => {
                *last_http_send_at = Some(Instant::now());
                return Ok(());
            }
            Err(TelegramSendError::Permanent(message)) => {
                *last_http_send_at = Some(Instant::now());
                return Err(AppError::downstream(message));
            }
            Err(TelegramSendError::Retriable {
                message,
                retry_after_ms,
            }) => {
                *last_http_send_at = Some(Instant::now());

                if attempt_index + 1 >= total_attempts {
                    return Err(AppError::downstream(message));
                }

                // On respecte `Retry-After` si Telegram le fournit; sinon on applique un backoff simple.
                let delay_ms = retry_after_ms
                    .unwrap_or(state.config.telegram.retry_backoff_ms * (attempt_index as u64 + 1));
                warn!(
                    attempt = attempt_index + 1,
                    total_attempts, delay_ms, chat_id, "telegram send failed, scheduling retry"
                );
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    Ok(())
}
