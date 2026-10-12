//! Endpoint WebSocket direct porte par Bunny pour le pilote DEV.

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::broadcast::error::RecvError,
    time::{Duration, MissedTickBehavior, interval},
};
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::live::{
        dto::{
            EdgeLiveHeartbeatPayload, EdgeLiveHelloPayload, EdgeLiveLagPayload, EdgeLiveMessage,
        },
        fanout::empty_snapshot,
    },
    shared::time::now_rfc3339,
};

use super::{HEARTBEAT_INTERVAL_SECS, serialization::serialize_live_metadata_message};

/// Garde le contrat Durable Object: `hello` contient le snapshot avant les événements live.
pub(super) async fn handle_edge_live_socket(socket: WebSocket, state: AppState) {
    let snapshot = match &state.live_fanout {
        Some(fanout) => match fanout.read_snapshot().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                warn!(%error, "live Redis snapshot unavailable; opening WebSocket with empty snapshot");
                empty_snapshot()
            }
        },
        None => empty_snapshot(),
    };
    let (mut sender, mut receiver) = socket.split();
    let hello = EdgeLiveMessage::Hello {
        payload: EdgeLiveHelloPayload {
            protocol_version: 1,
            server_time: now_rfc3339(),
            snapshot,
        },
    };
    let Some(hello_payload) = serialize_live_metadata_message(hello) else {
        return;
    };
    if sender
        .send(Message::Text(hello_payload.into()))
        .await
        .is_err()
    {
        return;
    }

    let mut live_rx = state.live_metadata_tx.subscribe();
    let mut heartbeat_ticker = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
    heartbeat_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    heartbeat_ticker.tick().await;

    loop {
        tokio::select! {
            incoming = receiver.next() => match incoming {
                Some(Ok(Message::Text(value))) if value == "ping" => {
                    if sender.send(Message::Text("pong".into())).await.is_err() { break; }
                }
                Some(Ok(Message::Text(value))) if value == "heartbeat" => {
                    let heartbeat = EdgeLiveMessage::Heartbeat { payload: EdgeLiveHeartbeatPayload { server_time: now_rfc3339() } };
                    let Some(payload) = serialize_live_metadata_message(heartbeat) else { continue; };
                    if sender.send(Message::Text(payload.into())).await.is_err() { break; }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
            _ = heartbeat_ticker.tick() => {
                let heartbeat = EdgeLiveMessage::Heartbeat { payload: EdgeLiveHeartbeatPayload { server_time: now_rfc3339() } };
                let Some(payload) = serialize_live_metadata_message(heartbeat) else { continue; };
                if sender.send(Message::Text(payload.into())).await.is_err() { break; }
            },
            received = live_rx.recv() => match received {
                Ok(message) => {
                    let Some(payload) = serialize_live_metadata_message(message) else { continue; };
                    if sender.send(Message::Text(payload.into())).await.is_err() { break; }
                }
                Err(RecvError::Lagged(skipped)) => {
                    warn!(skipped, "Bunny live WebSocket subscriber lagged");
                    let lagged = EdgeLiveMessage::Lagged { payload: EdgeLiveLagPayload { server_time: now_rfc3339(), skipped } };
                    let Some(payload) = serialize_live_metadata_message(lagged) else { continue; };
                    if sender.send(Message::Text(payload.into())).await.is_err() { break; }
                }
                Err(RecvError::Closed) => {
                    info!("Bunny live WebSocket closed because broadcast bus closed");
                    break;
                }
            }
        }
    }
}
