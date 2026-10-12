//! Boucle de lecture/ecriture du WebSocket de trace E2E.

mod client;
mod trace;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{
    StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::{sync::broadcast::Receiver, time::Interval};

use crate::{app_state::AppState, features::live::dto::TestTraceEvent};

use super::messages::send_test_trace_heartbeat;
use client::handle_client_message;
use trace::handle_trace_message;

/// Relaye commandes client, heartbeats et traces broadcast jusqu'a fermeture du socket.
pub(super) async fn run_test_trace_stream(
    sender: &mut SplitSink<WebSocket, Message>,
    receiver: &mut SplitStream<WebSocket>,
    state: &AppState,
    trace_rx: &mut Receiver<TestTraceEvent>,
    heartbeat_ticker: &mut Interval,
) {
    loop {
        tokio::select! {
            client_message = receiver.next() => {
                if !handle_client_message(sender, state, client_message).await {
                    break;
                }
            },
            _ = heartbeat_ticker.tick() => {
                if send_test_trace_heartbeat(sender).await.is_err() {
                    break;
                }
            }
            trace_message = trace_rx.recv() => {
                if !handle_trace_message(sender, trace_message).await {
                    break;
                }
            }
        }
    }
}
