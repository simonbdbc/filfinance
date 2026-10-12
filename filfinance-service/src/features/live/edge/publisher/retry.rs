//! Politique de retry de publication vers le Worker edge.

use std::time::Duration;

use serde::Serialize;
use tokio::time::sleep;
use tracing::warn;

use crate::errors::AppError;

use super::{EdgePublisher, error::EdgePublishError};

impl EdgePublisher {
    /// Publie un message edge avec un nombre borne de retries.
    pub(in crate::features::live::edge) async fn publish_with_retry<TMessage>(
        &self,
        message: &TMessage,
    ) -> Result<(), AppError>
    where
        // Le message reste generique pour tester la politique sans dupliquer la logique HTTP.
        TMessage: Serialize + ?Sized,
    {
        // Comme Telegram, `publish_retry_attempts` compte seulement les retries apres la premiere tentative.
        let total_attempts = self.config.publish_retry_attempts + 1;

        for attempt_index in 0..total_attempts {
            match self.publish_once(message).await {
                Ok(()) => return Ok(()),
                Err(EdgePublishError::Permanent(message)) => {
                    return Err(AppError::downstream(message));
                }
                Err(EdgePublishError::Retriable(message)) => {
                    if attempt_index + 1 >= total_attempts {
                        return Err(AppError::downstream(message));
                    }

                    // Backoff lineaire simple: lisible, borne et suffisant pour un Worker edge.
                    let delay_ms =
                        self.config.publish_retry_backoff_ms * (attempt_index as u64 + 1);
                    warn!(
                        attempt = attempt_index + 1,
                        total_attempts, delay_ms, "edge publish failed, scheduling retry"
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        // La boucle retourne normalement avant cette ligne; ce `Ok` garde le compilateur satisfait.
        Ok(())
    }
}
