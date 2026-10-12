//! Transport texte commun aux WebSockets backend.

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, stream::SplitSink};

/// Envoie un payload texte déjà sérialisé vers le socket cible.
pub(super) async fn send_text(
    sender: &mut SplitSink<WebSocket, Message>,
    payload: String,
) -> Result<(), ()> {
    // `payload.into()` convertit la `String` possédée vers le type interne de `Message::Text`.
    sender
        .send(Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}
