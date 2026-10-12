//! Fermeture policy des WebSockets refuses par l'auth.

use axum::extract::ws::{CloseFrame, Message, WebSocket, close_code};

/// Ferme le socket avec un code policy violation sans divulguer de detail sensible.
pub(super) async fn close_socket_with_policy(socket: &mut WebSocket, reason: &'static str) {
    // `1008` signale au client un refus de politique/auth sans confondre avec une panne reseau.
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: close_code::POLICY,
            reason: reason.into(),
        })))
        .await;
}
