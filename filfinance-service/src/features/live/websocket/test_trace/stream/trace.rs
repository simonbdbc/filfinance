//! Evenements broadcast envoyes au WebSocket de trace E2E.

use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;
use tokio::sync::broadcast::error::RecvError;
use tracing::{info, warn};

use crate::features::live::dto::TestTraceEvent;

use super::super::messages::{send_test_trace_event, send_test_trace_lagged};

/// Traite un evenement de trace broadcast et indique si la boucle doit rester ouverte.
pub(super) async fn handle_trace_message(
    sender: &mut SplitSink<WebSocket, Message>,
    trace_message: Result<TestTraceEvent, RecvError>,
) -> bool {
    match trace_message {
        Ok(message) => {
            // Chaque trace de test reste un message JSON simple pour etre lisible dans le navigateur.
            send_test_trace_event(sender, message).await.is_ok()
        }
        Err(RecvError::Lagged(skipped)) => {
            warn!(
                skipped,
                stream = "test_trace",
                "websocket subscriber lagged behind test trace stream"
            );

            send_test_trace_lagged(sender, skipped).await.is_ok()
        }
        Err(RecvError::Closed) => {
            info!(stream = "test_trace", "websocket test trace stream closed");
            false
        }
    }
}
