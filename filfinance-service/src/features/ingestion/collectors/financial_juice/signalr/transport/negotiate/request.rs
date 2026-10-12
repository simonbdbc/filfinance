//! Appels HTTP de négociation SignalR FinancialJuice.

// `Duration` borne les appels HTTP de négociation.
use std::time::Duration;

use crate::{
    errors::AppError,
    features::ingestion::collectors::financial_juice::{
        FINANCIAL_JUICE_SIGNALR_CLIENT_PROTOCOL, FINANCIAL_JUICE_SIGNALR_CONNECTION_DATA,
        FinancialJuiceSiteCollector, build_url_with_query, ensure_success_response,
        model::FinancialJuiceBootstrap,
    },
};

use super::super::super::dto::{
    FinancialJuiceSignalrAppNegotiateResponse, FinancialJuiceSignalrTransportNegotiateResponse,
};

impl FinancialJuiceSiteCollector {
    /// Appelle la négociation applicative exposée par FinancialJuice.
    pub(super) async fn request_app_negotiate(
        &self,
        bootstrap: &FinancialJuiceBootstrap,
    ) -> Result<FinancialJuiceSignalrAppNegotiateResponse, AppError> {
        // Le premier endpoint est porté par l'application FinancialJuice elle-même.
        let negotiate_url = format!("{}/signalr/negotiate", bootstrap.main_url);
        let response = self
            .client
            .get(build_url_with_query(
                &negotiate_url,
                &signalr_negotiate_params(&bootstrap.feedtoken),
            )?)
            .timeout(Duration::from_millis(self.config.request_timeout_ms))
            .send()
            .await
            .map_err(|error| AppError::fetch(self.config.name.clone(), error.to_string()))
            .and_then(|response| ensure_success_response(&self.config.name, response))?
            .json::<FinancialJuiceSignalrAppNegotiateResponse>()
            .await?;

        Ok(response)
    }

    /// Appelle la négociation finale après redirection Azure SignalR.
    pub(super) async fn request_transport_negotiate(
        &self,
        bootstrap: &FinancialJuiceBootstrap,
        redirect_url: &str,
        access_token: &str,
    ) -> Result<FinancialJuiceSignalrTransportNegotiateResponse, AppError> {
        // `trim_end_matches('/')` évite de fabriquer des URLs avec double slash.
        let transport_base_url = redirect_url.trim_end_matches('/');
        let response = self
            .client
            .get(build_url_with_query(
                &format!("{transport_base_url}/negotiate"),
                &signalr_negotiate_params(&bootstrap.feedtoken),
            )?)
            .timeout(Duration::from_millis(self.config.request_timeout_ms))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| AppError::fetch(self.config.name.clone(), error.to_string()))
            .and_then(|response| ensure_success_response(&self.config.name, response))?
            .json::<FinancialJuiceSignalrTransportNegotiateResponse>()
            .await?;

        Ok(response)
    }
}

/// Paramètres communs aux deux endpoints `negotiate` SignalR.
fn signalr_negotiate_params(feedtoken: &str) -> [(&'static str, String); 3] {
    // Le protocole et le hub sont constants; seul le `feedtoken` vient du bootstrap authentifié.
    [
        (
            "clientProtocol",
            FINANCIAL_JUICE_SIGNALR_CLIENT_PROTOCOL.to_owned(),
        ),
        (
            "connectionData",
            FINANCIAL_JUICE_SIGNALR_CONNECTION_DATA.to_owned(),
        ),
        ("ftoken", feedtoken.to_owned()),
    ]
}
