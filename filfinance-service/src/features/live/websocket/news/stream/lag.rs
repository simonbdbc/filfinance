//! Message de lag commun aux flux broadcast du WebSocket news.

use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;

use crate::features::live::dto::WsServerMessage;

use super::super::super::{send_text, serialize_message};

/// Envoie le message protocolaire indiquant un lag broadcast cote serveur.
pub(super) async fn send_lagged(
    sender: &mut SplitSink<WebSocket, Message>,
    skipped: u64,
) -> Result<(), ()> {
    let Some(payload) = serialize_message(WsServerMessage::lagged(skipped)) else {
        return Ok(());
    };

    send_text(sender, payload).await
}
