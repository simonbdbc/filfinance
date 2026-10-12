//! Emission des messages du WebSocket de trace E2E.

use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;

use crate::features::live::dto::{TestTraceEvent, WsTestTraceMessage};

use super::super::{send_text, serialize_test_trace_message};

/// Envoie le message initial du flux de trace.
pub(super) async fn send_test_trace_hello(
    sender: &mut SplitSink<WebSocket, Message>,
    broadcast_buffer: usize,
) -> Result<(), ()> {
    send_test_trace_message(sender, WsTestTraceMessage::hello(broadcast_buffer)).await
}

/// Envoie un heartbeat de maintien du canal de debug.
pub(super) async fn send_test_trace_heartbeat(
    sender: &mut SplitSink<WebSocket, Message>,
) -> Result<(), ()> {
    send_test_trace_message(sender, WsTestTraceMessage::heartbeat()).await
}

/// Envoie une trace de pipeline au client de debug.
pub(super) async fn send_test_trace_event(
    sender: &mut SplitSink<WebSocket, Message>,
    message: TestTraceEvent,
) -> Result<(), ()> {
    send_test_trace_message(sender, WsTestTraceMessage::test_trace(message)).await
}

/// Envoie un signal de lag quand le receiver broadcast a pris du retard.
pub(super) async fn send_test_trace_lagged(
    sender: &mut SplitSink<WebSocket, Message>,
    skipped: u64,
) -> Result<(), ()> {
    send_test_trace_message(sender, WsTestTraceMessage::lagged(skipped)).await
}

/// Sérialise puis transmet un message de trace si la conversion JSON réussit.
async fn send_test_trace_message(
    sender: &mut SplitSink<WebSocket, Message>,
    message: WsTestTraceMessage,
) -> Result<(), ()> {
    let Some(payload) = serialize_test_trace_message(message) else {
        return Ok(());
    };

    send_text(sender, payload).await
}
