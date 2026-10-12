//! Negociation du transport SignalR FinancialJuice.

// Les sous-modules separent les appels HTTP et la construction de session finale.
mod request;
mod session;

use crate::{
    errors::AppError,
    features::ingestion::collectors::financial_juice::{
        FinancialJuiceSiteCollector, model::FinancialJuiceBootstrap,
    },
};

use super::super::dto::FinancialJuiceSignalrTransportSession;

impl FinancialJuiceSiteCollector {
    /// Negocie une session de transport live utilisable par SignalR 2.x.
    pub(in crate::features::ingestion::collectors::financial_juice) async fn negotiate_signalr_transport(
        &self,
        bootstrap: &FinancialJuiceBootstrap,
    ) -> Result<FinancialJuiceSignalrTransportSession, AppError> {
        // Premiere etape: negociation cote application FinancialJuice.
        let first_response = self.request_app_negotiate(bootstrap).await?;

        // Le cas principal observe en prod est une redirection Azure SignalR avec bearer token.
        if let (Some(redirect_url), Some(access_token)) =
            (&first_response.redirect_url, &first_response.access_token)
        {
            // Deuxieme etape: negociation cote transport final Azure SignalR.
            let second_response = self
                .request_transport_negotiate(bootstrap, redirect_url, access_token)
                .await?;

            // `return Ok(...)` sort explicitement de cette branche de protocole, ce qui garde le reste
            // de la fonction reserve au cas fallback sans `else` gigantesque.
            return Ok(session::redirected_transport_session(
                redirect_url,
                access_token,
                second_response,
                &bootstrap.feedtoken,
            ));
        }

        // Fallback plus simple: certains serveurs SignalR peuvent renvoyer directement un `connectionToken`
        // sans passer par la redirection Azure observee sur le site actuel.
        if let Some(connection_token) = first_response.connection_token {
            return Ok(session::direct_transport_session(
                &bootstrap.main_url,
                first_response.url,
                connection_token,
                &bootstrap.feedtoken,
            ));
        }

        Err(session::unusable_transport_error(&self.config.name))
    }
}
