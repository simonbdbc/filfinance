//! Messages initiaux et heartbeat du WebSocket news.

// `Message` et `WebSocket` typent la demi-socket d'ecriture.
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;

use crate::{
    app_state::AppState,
    features::{
        calendar_day::public_calendar_day_summary_status, live::dto::WsServerMessage,
        news::status::public_source_statuses,
    },
};

use super::super::super::{send_text, serialize_message};

/// Envoie le message `hello` qui fixe le contrat de protocole des la connexion.
pub(super) async fn send_hello(
    sender: &mut SplitSink<WebSocket, Message>,
    state: &AppState,
) -> Result<(), ()> {
    let Some(payload) = serialize_message(WsServerMessage::hello(
        state.config.runtime.broadcast_buffer,
        public_source_statuses(state),
        public_calendar_day_summary_status(state).await,
    )) else {
        return Ok(());
    };

    send_text(sender, payload).await
}

/// Envoie un heartbeat applicatif sur la socket user-facing.
pub(super) async fn send_heartbeat(
    sender: &mut SplitSink<WebSocket, Message>,
    state: &AppState,
) -> Result<(), ()> {
    let Some(payload) = serialize_message(WsServerMessage::heartbeat(
        public_source_statuses(state),
        public_calendar_day_summary_status(state).await,
    )) else {
        return Ok(());
    };

    send_text(sender, payload).await
}
