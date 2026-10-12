//! Consommation des frames live SignalR FinancialJuice.

use futures_util::{Stream, StreamExt};
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};

use crate::{
    app_state::AppState, errors::AppError, features::ingestion::collectors::emit_items_to_pipeline,
};

use super::super::super::FinancialJuiceSiteCollector;

/// Lit les frames live jusqu'a fermeture du WebSocket ou arret du pipeline d'ingestion.
pub(super) async fn consume_live_frames<S>(
    collector: &FinancialJuiceSiteCollector,
    websocket: &mut S,
    state: &AppState,
) -> Result<(), AppError>
where
    S: Stream<Item = Result<Message, TungsteniteError>> + Unpin,
{
    // `while let Some(...)` suit naturellement le flux d'un WebSocket: on lit jusqu'à fermeture.
    while let Some(frame) = websocket.next().await {
        if !state.financial_juice_runtime.is_enabled() {
            // Le switch admin doit couper le hot path au plus tard a la prochaine frame recue.
            return Ok(());
        }

        match frame {
            Ok(Message::Text(text)) => {
                // Toute frame texte est d'abord décodée en lot de `NewsEvent` canoniques.
                let items = collector.extract_news_events_from_live_frame(text.as_str())?;
                // L'émission vers le pipeline renvoie `false` si l'ingestion est déjà fermée;
                // on sort alors proprement sans considérer cela comme une erreur amont.
                if !emit_items_to_pipeline(&collector.config.name, items, &state.ingest_tx).await {
                    return Ok(());
                }
            }
            Ok(Message::Binary(_)) => {
                // Le protocole observé est purement texte; on ignore les frames binaires inattendues.
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => {
                return Ok(());
            }
            Ok(Message::Frame(_)) => {}
            Err(error) => {
                return Err(AppError::downstream(format!(
                    "signalr websocket read failed: {error}"
                )));
            }
        }
    }

    Ok(())
}
