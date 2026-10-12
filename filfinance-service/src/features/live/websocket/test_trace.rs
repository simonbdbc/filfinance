//! WebSocket de debug pour les traces et commandes de run de test.

mod command;
mod messages;
mod stream;
mod trigger;

use axum::extract::ws::WebSocket;
use futures_util::{SinkExt, StreamExt};
use tokio::time::{Duration, MissedTickBehavior, interval};

use super::{HEARTBEAT_INTERVAL_SECS, authenticate_socket};
use crate::app_state::AppState;
use messages::send_test_trace_hello;
use stream::run_test_trace_stream;

/// Pousse les traces de run de test et accepte les commandes du client de debug.
pub(super) async fn handle_test_trace_socket(mut socket: WebSocket, state: AppState) {
    // Le canal test-runs permet de declencher une injection synthetique; il exige un role moderator.
    let user = match authenticate_socket(&mut socket, &state, "test_runs", true).await {
        Ok(user) => user,
        Err(()) => return,
    };
    let access_token_expires_at = user.access_token_expires_at;

    // Ce socket isole le debug E2E du flux user-facing pour ne pas polluer le contrat client final.
    let (mut sender, mut receiver) = socket.split();
    let mut trace_rx = state.test_trace_tx.subscribe();
    let mut heartbeat_ticker = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
    heartbeat_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let completed = crate::features::auth::complete_before_access_token_expiration(
        access_token_expires_at,
        async {
            if send_test_trace_hello(&mut sender, state.config.runtime.broadcast_buffer)
                .await
                .is_err()
            {
                return;
            }

            heartbeat_ticker.tick().await;
            run_test_trace_stream(
                &mut sender,
                &mut receiver,
                &state,
                &mut trace_rx,
                &mut heartbeat_ticker,
            )
            .await;
        },
    )
    .await;

    if completed.is_none() {
        let _ = sender
            .send(crate::features::auth::access_token_expired_close_message())
            .await;
    }
}
