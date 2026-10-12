//! Frames client recues par le WebSocket de trace E2E.

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, stream::SplitSink};
use tracing::{info, warn};

use crate::app_state::AppState;

use super::super::command::handle_test_trace_command;

/// Traite une frame client et indique si la boucle doit rester ouverte.
pub(super) async fn handle_client_message(
    sender: &mut SplitSink<WebSocket, Message>,
    state: &AppState,
    client_message: Option<Result<Message, axum::Error>>,
) -> bool {
    match client_message {
        Some(Ok(Message::Close(frame))) => {
            info!(?frame, "websocket test trace client closed connection");
            false
        }
        Some(Ok(Message::Ping(payload))) => sender.send(Message::Pong(payload)).await.is_ok(),
        Some(Ok(Message::Pong(_))) => true,
        Some(Ok(Message::Text(payload))) => {
            // Le debug accepte des commandes texte minimales, typiquement `trigger_test_run`.
            handle_test_trace_command(sender, state, payload.as_str())
                .await
                .is_ok()
        }
        Some(Ok(Message::Binary(_))) => true,
        Some(Err(error)) => {
            warn!(%error, "websocket test trace client stream failed");
            false
        }
        None => {
            info!("websocket test trace client stream ended");
            false
        }
    }
}
