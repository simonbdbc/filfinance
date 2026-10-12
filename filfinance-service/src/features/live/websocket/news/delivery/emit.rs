//! Emission et trace des items WebSocket user-facing.

use axum::extract::ws::{Message, WebSocket};
// `SplitSink` est la moitié "écriture" retournée par `socket.split()`.
use futures_util::stream::SplitSink;

use crate::{
    app_state::AppState,
    features::live::dto::{LiveNewsItem, WsServerMessage},
};

use super::super::super::{send_text, serialize_message};

/// Envoie un `LiveNewsItem` à un client puis trace explicitement le delivery du run de test.
pub(in crate::features::live::websocket::news) async fn emit_live_item(
    sender: &mut SplitSink<WebSocket, Message>,
    state: &AppState,
    live_item: LiveNewsItem,
    success_stage: &str,
    success_message: &str,
) -> Result<(), ()> {
    let detail = Some(format!("news_event_id={}", live_item.news_event_id));
    let test_run_id = live_item.test_run_id.clone();
    // On sérialise le payload avant tout effet de trace: un échec JSON n'est pas un crash de socket.
    let payload = match serialize_message(WsServerMessage::translated_news_item(live_item)) {
        Some(payload) => payload,
        None => {
            if let Some(run_id) = test_run_id.as_deref() {
                state.emit_test_trace(
                    run_id,
                    "user_ws_serialization_failed",
                    "error",
                    "La serialisation du payload WebSocket user-facing a echoue.",
                    detail,
                );
            }
            return Ok(());
        }
    };

    if send_text(sender, payload).await.is_err() {
        if let Some(run_id) = test_run_id.as_deref() {
            state.emit_test_trace(
                run_id,
                "user_ws_send_failed",
                "error",
                "L'envoi au WebSocket user-facing a echoue.",
                detail,
            );
        }
        return Err(());
    }

    if let Some(run_id) = test_run_id.as_deref() {
        state.emit_test_trace(run_id, success_stage, "ok", success_message, detail);
    }

    Ok(())
}
