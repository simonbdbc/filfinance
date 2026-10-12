//! Demarrage HTTP du transport SignalR FinancialJuice.

// `Duration` borne l'appel HTTP `start`.
use std::time::Duration;

use crate::{
    errors::AppError,
    features::ingestion::collectors::financial_juice::{
        FINANCIAL_JUICE_SIGNALR_CLIENT_PROTOCOL, FINANCIAL_JUICE_SIGNALR_CONNECTION_DATA,
        FinancialJuiceSiteCollector, build_url_with_query, ensure_success_response,
    },
};

use super::super::dto::FinancialJuiceSignalrTransportSession;

impl FinancialJuiceSiteCollector {
    /// Finalise le transport live via l'appel HTTP `start`.
    pub(in crate::features::ingestion::collectors::financial_juice) async fn start_signalr_transport(
        &self,
        session: &FinancialJuiceSignalrTransportSession,
    ) -> Result<(), AppError> {
        // On demarre la session HTTP apres le handshake WebSocket car c'est l'ordre attendu par ce serveur.
        let mut request = self
            .client
            .get(build_url_with_query(
                &format!("{}/start", session.transport_base_url),
                &[
                    ("transport", "webSockets".to_owned()),
                    (
                        "clientProtocol",
                        FINANCIAL_JUICE_SIGNALR_CLIENT_PROTOCOL.to_owned(),
                    ),
                    ("connectionToken", session.connection_token.clone()),
                    (
                        "connectionData",
                        FINANCIAL_JUICE_SIGNALR_CONNECTION_DATA.to_owned(),
                    ),
                    ("ftoken", session.feedtoken.clone()),
                ],
            )?)
            .timeout(Duration::from_millis(self.config.request_timeout_ms));

        // Le transport Azure exige un bearer token, alors que le mode direct n'en a pas besoin.
        if let Some(access_token) = session.access_token.as_deref() {
            request = request.bearer_auth(access_token);
        }

        // On ne lit pas le corps ici: seul le statut HTTP nous interesse pour savoir si le transport est pret.
        request
            .send()
            .await
            .map_err(|error| AppError::downstream(format!("signalr start failed: {error}")))
            .and_then(|response| {
                ensure_success_response(&self.config.name, response)
                    .map_err(|error| AppError::downstream(format!("signalr start failed: {error}")))
            })?;

        Ok(())
    }
}
