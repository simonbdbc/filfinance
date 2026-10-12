//! Borne temporelle commune aux transports WebSocket authentifies.

use std::{
    future::{Future, pending},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::extract::ws::{CloseFrame, Message, close_code};

/// Execute une operation seulement tant que le JWT qui a ouvert le socket reste valide.
pub(crate) async fn complete_before_access_token_expiration<F>(
    expires_at: Option<u64>,
    operation: F,
) -> Option<F::Output>
where
    F: Future,
{
    let expiration_delay = access_token_expiration_delay_at(expires_at, SystemTime::now());
    // Un `sleep(0)` Tokio peut rester Pending a son premier poll: on refuse donc ce cas synchronement.
    if expiration_delay == Some(Duration::ZERO) {
        return None;
    }
    // L'expiration gagne si les deux branches deviennent pretes au meme instant.
    tokio::select! {
        biased;
        _ = wait_for_access_token_expiration(expiration_delay) => None,
        output = operation => Some(output),
    }
}

/// Frame commune envoyee quand le claim `exp` interrompt une connexion persistante.
pub(crate) fn access_token_expired_close_message() -> Message {
    Message::Close(Some(CloseFrame {
        code: close_code::POLICY,
        reason: "auth token expired".into(),
    }))
}

async fn wait_for_access_token_expiration(expiration_delay: Option<Duration>) {
    let Some(delay) = expiration_delay else {
        // Le mode local avec auth desactivee conserve les WebSockets sans echeance artificielle.
        pending::<()>().await;
        return;
    };
    tokio::time::sleep(delay).await;
}

fn access_token_expiration_delay_at(expires_at: Option<u64>, now: SystemTime) -> Option<Duration> {
    let expires_at = expires_at?;
    // Une date hors plage ferme par securite au lieu de laisser un socket sans borne.
    let expiration = UNIX_EPOCH
        .checked_add(Duration::from_secs(expires_at))
        .unwrap_or(now);
    Some(expiration.duration_since(now).unwrap_or(Duration::ZERO))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use axum::{
        Router,
        extract::ws::{Message, WebSocketUpgrade, close_code},
        response::Response,
        routing::get,
    };
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::{
        Message as ClientMessage, protocol::frame::coding::CloseCode,
    };

    use super::{
        access_token_expiration_delay_at, access_token_expired_close_message,
        complete_before_access_token_expiration,
    };

    #[test]
    fn expiration_delay_is_zero_at_or_after_exp() {
        let now = UNIX_EPOCH + Duration::from_secs(1_800_000_000);

        assert_eq!(
            access_token_expiration_delay_at(Some(1_800_000_000), now),
            Some(Duration::ZERO)
        );
        assert_eq!(
            access_token_expiration_delay_at(Some(1_799_999_999), now),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn expiration_delay_preserves_future_deadline_and_auth_off() {
        let now = UNIX_EPOCH + Duration::from_secs(1_800_000_000);

        assert_eq!(
            access_token_expiration_delay_at(Some(1_800_000_010), now),
            Some(Duration::from_secs(10))
        );
        assert_eq!(access_token_expiration_delay_at(None, now), None);
    }

    #[test]
    fn expired_token_uses_policy_close_frame() {
        let Message::Close(Some(frame)) = access_token_expired_close_message() else {
            panic!("token expiration should produce a close frame");
        };

        assert_eq!(frame.code, close_code::POLICY);
        assert_eq!(frame.reason, "auth token expired");
    }

    #[tokio::test]
    async fn expired_deadline_preempts_even_an_immediately_ready_operation() {
        let completed =
            complete_before_access_token_expiration(Some(0), async { "protected" }).await;

        assert_eq!(completed, None);
    }

    #[tokio::test]
    async fn auth_off_preserves_immediately_ready_operation() {
        let completed = complete_before_access_token_expiration(None, async { "local" }).await;

        assert_eq!(completed, Some("local"));
    }

    #[tokio::test]
    async fn expired_deadline_closes_wire_before_protected_frame() {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("test websocket listener should bind");
        let address = listener
            .local_addr()
            .expect("test websocket listener should expose its address");
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().route("/", get(expired_test_socket)))
                .await
                .expect("test websocket server should run");
        });
        let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{address}/"))
            .await
            .expect("test websocket client should connect");

        let message = tokio::time::timeout(Duration::from_secs(1), client.next())
            .await
            .expect("expired websocket should close promptly")
            .expect("expired websocket should emit a close frame")
            .expect("expired websocket frame should be readable");

        let ClientMessage::Close(Some(frame)) = message else {
            panic!("expired websocket must close before any protected frame: {message:?}");
        };
        assert_eq!(frame.code, CloseCode::Policy);
        assert_eq!(frame.reason, "auth token expired");
        server.abort();
    }

    async fn expired_test_socket(ws: WebSocketUpgrade) -> Response {
        ws.on_upgrade(|socket| async move {
            let (mut sender, _) = socket.split();
            let completed = complete_before_access_token_expiration(
                Some(0),
                sender.send(Message::Text("protected".into())),
            )
            .await;

            if completed.is_none() {
                let _ = sender.send(access_token_expired_close_message()).await;
            }
        })
    }
}
