//! Traduction et shutdown de la session locale long-vivante.

use tokio::time::{Duration, timeout};
use tracing::info;

use crate::{
    errors::AppError,
    features::translation::domain::{ProviderTranslation, TranslationRequest},
};

use super::super::{
    request::{PreparedSessionRequest, prepare_session_request},
    response::restore_session_translation,
};
use super::LocalCommandSessionTranslationProvider;

impl LocalCommandSessionTranslationProvider {
    /// Traduit un item via une session stdin/stdout gardee en memoire.
    pub(crate) async fn translate(
        &mut self,
        request: &TranslationRequest,
    ) -> Result<ProviderTranslation, AppError> {
        let PreparedSessionRequest {
            payload_line,
            protected_title,
            protected_body,
        } = prepare_session_request(request)?;
        // La session n'est demarree qu'au premier besoin reel, pour garder un cold start leger.
        self.ensure_session_started().await?;

        // `timeout(...)` borne l'echange complet requete/reponse avec le helper.
        let response = match timeout(
            Duration::from_millis(self.timeout_ms),
            self.exchange_line(payload_line),
        )
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                // En cas d'erreur protocolaire, on jette la session complete pour repartir proprement.
                self.reset_session().await;
                return Err(error);
            }
            Err(_) => {
                self.reset_session().await;
                return Err(AppError::downstream(format!(
                    "translation session timed out after {}ms",
                    self.timeout_ms
                )));
            }
        };

        restore_session_translation(
            response,
            protected_title,
            protected_body,
            &self.provider_name,
        )
    }

    /// Termine explicitement le helper long-vivant avant l'arret du worker.
    pub(crate) async fn shutdown(&mut self) {
        self.reset_session().await;
        info!(
            provider = self.provider_name,
            "local translation session provider shut down cleanly"
        );
    }
}
