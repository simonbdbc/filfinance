//! Lecture du premier message d'authentification WebSocket.

use axum::extract::ws::{Message, WebSocket};

use super::command::parse_websocket_auth_token;

/// Lit le premier message d'auth en tolerant les ping/pong techniques avant celui-ci.
pub(super) async fn read_websocket_auth_token(
    socket: &mut WebSocket,
) -> Result<String, &'static str> {
    loop {
        // `recv()` renvoie `None` si le client ferme avant de s'authentifier.
        let message = socket.recv().await.ok_or("auth socket closed")?;
        // Une erreur de transport avant auth coupe le handshake.
        let message = message.map_err(|_| "auth transport error")?;

        match message {
            Message::Text(payload) => {
                // Le parsing est isole dans un helper testable sans ouvrir de socket reel.
                return parse_websocket_auth_token(payload.as_str());
            }
            Message::Ping(payload) => {
                // Les ping navigateur/proxy ne doivent pas faire echouer le handshake.
                if socket.send(Message::Pong(payload)).await.is_err() {
                    return Err("auth pong failed");
                }
            }
            Message::Pong(_) => {
                // Un pong isole avant auth est ignore.
            }
            Message::Close(_) => return Err("auth socket closed"),
            Message::Binary(_) => return Err("auth message must be text"),
        }
    }
}
