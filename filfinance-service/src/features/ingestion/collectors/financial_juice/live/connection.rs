//! Cycle WebSocket SignalR du live FinancialJuice.

mod frames;
mod initialization;

// `connect_async` ouvre le WebSocket SignalR côté client.
use tokio_tungstenite::connect_async;
// Les logs structurés rendent lisibles les phases live et les reconnexions.
use tracing::debug;

use crate::{app_state::AppState, errors::AppError};

use super::super::{
    FINANCIAL_JUICE_MAIN_FEED_ID, FinancialJuiceSiteCollector, signalr::build_signalr_connect_url,
};
use frames::consume_live_frames;
use initialization::read_initial_signalr_frame;

impl FinancialJuiceSiteCollector {
    /// Exécute un cycle complet de connexion live jusqu'à fermeture du socket.
    pub(in crate::features::ingestion::collectors::financial_juice::live) async fn run_live_once(
        &self,
        state: &AppState,
    ) -> Result<(), AppError> {
        // Le live réutilise la session bootstrapée plutôt que de relogger à chaque reconnexion.
        let bootstrap = self.ensure_bootstrap(false).await?;
        debug!(
            source = %self.config.name,
            feed_id = FINANCIAL_JUICE_MAIN_FEED_ID,
            allowed_feed_ids = ?bootstrap.feed_ids,
            "financial juice authenticated bootstrap ready"
        );
        // La négociation SignalR encapsule la complexité "app negotiate -> Azure negotiate".
        let transport = self.negotiate_signalr_transport(&bootstrap).await?;
        // L'URL WebSocket finale dépend de la négociation précédente et peut inclure un access token.
        let ws_url = build_signalr_connect_url(&transport)?;
        // `connect_async` suffit ici: l'auth passe déjà dans la query string `access_token`.
        let (mut websocket, _) = connect_async(ws_url.as_str()).await.map_err(|error| {
            AppError::downstream(format!("signalr websocket connect failed: {error}"))
        })?;

        // Le premier message SignalR attendu est l'accusé d'initialisation `{"S":1,"M":[]}`.
        read_initial_signalr_frame(&mut websocket).await?;

        self.start_signalr_transport(&transport).await?;

        consume_live_frames(self, &mut websocket, state).await
    }
}
