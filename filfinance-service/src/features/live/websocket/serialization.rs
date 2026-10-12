//! Serialisation JSON des messages WebSocket live.

use tracing::warn;

use crate::features::live::dto::{EdgeLiveMessage, WsServerMessage, WsTestTraceMessage};

/// Serialise un message user-facing en JSON sans propager une erreur bloquante au flux.
pub(in crate::features::live) fn serialize_message(message: WsServerMessage) -> Option<String> {
    match serde_json::to_string(&message) {
        Ok(payload) => Some(payload),
        Err(error) => {
            warn!(%error, "failed to serialize websocket message");
            None
        }
    }
}

/// Serialise un message live edge-compatible pour le fallback backend `/ws/news`.
pub(in crate::features::live) fn serialize_live_metadata_message(
    message: EdgeLiveMessage,
) -> Option<String> {
    match serde_json::to_string(&message) {
        Ok(payload) => Some(payload),
        Err(error) => {
            warn!(%error, "failed to serialize websocket live metadata message");
            None
        }
    }
}

/// Serialise un message du canal de trace en JSON sans casser le socket en cas d'erreur.
pub(in crate::features::live) fn serialize_test_trace_message(
    message: WsTestTraceMessage,
) -> Option<String> {
    match serde_json::to_string(&message) {
        Ok(payload) => Some(payload),
        Err(error) => {
            warn!(%error, "failed to serialize websocket test trace message");
            None
        }
    }
}
