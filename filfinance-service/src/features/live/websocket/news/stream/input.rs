//! Traitement des messages entrants du client WebSocket news.

// `Message` et `WebSocket` exposent les primitives du protocole live.
use axum::extract::ws::{Message, WebSocket};
// `SinkExt` permet d'ecrire le pong directement sur la socket splitee.
use futures_util::{SinkExt, stream::SplitSink};
// Les logs structures servent ici surtout a comprendre pourquoi une socket a coupe.
use tracing::{info, warn};

/// Traite un message client et indique si la boucle WebSocket doit continuer.
pub(super) async fn handle_client_message(
    sender: &mut SplitSink<WebSocket, Message>,
    client_message: Option<Result<Message, axum::Error>>,
) -> bool {
    match client_message {
        Some(Ok(Message::Close(frame))) => {
            info!(?frame, "websocket client closed connection");
            false
        }
        Some(Ok(Message::Ping(payload))) => sender.send(Message::Pong(payload)).await.is_ok(),
        Some(Ok(Message::Pong(_))) => true,
        // Le flux user-facing n'accepte pas de commandes texte pour le moment,
        // mais on les ignore silencieusement pour garder un protocole tolerant.
        Some(Ok(Message::Text(_))) => true,
        Some(Ok(Message::Binary(_))) => true,
        Some(Err(error)) => {
            warn!(%error, "websocket client stream failed");
            false
        }
        None => {
            info!("websocket client stream ended");
            false
        }
    }
}
