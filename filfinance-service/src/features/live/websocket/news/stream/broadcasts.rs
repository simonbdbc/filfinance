//! Branches broadcast de la boucle WebSocket news.

// `Message` et `WebSocket` typent la demi-socket d'ecriture.
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;
// `broadcast::Receiver::recv()` peut echouer en `Lagged` ou `Closed`; on modele ces cas explicitement.
use tokio::sync::broadcast::error::RecvError;
// Les logs structures distinguent les deux flux broadcast.
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::{live::dto::EdgeLiveMessage, translation::domain::PersistedNewsTranslation},
};

use super::super::super::{send_text, serialize_live_metadata_message};
use super::super::delivery::{build_live_news_item, emit_live_item};
use super::lag::send_lagged;

/// Traite une traduction diffusee par le worker de traduction.
pub(super) async fn handle_translated_broadcast(
    sender: &mut SplitSink<WebSocket, Message>,
    state: &AppState,
    translated_message: Result<PersistedNewsTranslation, RecvError>,
) -> bool {
    match translated_message {
        Ok(message) => {
            let live_item = build_live_news_item(state, message).await;
            emit_live_item(
                sender,
                state,
                live_item,
                "user_ws_emitted",
                "L'item traduit a ete envoye a un client WebSocket user-facing.",
            )
            .await
            .is_ok()
        }
        Err(RecvError::Lagged(skipped)) => {
            // Un receiver broadcast trop lent perd des messages; on prefere l'annoncer
            // plutot que tenter un rattrapage implicite impossible.
            warn!(
                skipped,
                stream = "translated_news",
                "websocket subscriber lagged behind translated broadcast stream"
            );

            send_lagged(sender, skipped).await.is_ok()
        }
        Err(RecvError::Closed) => {
            info!(
                stream = "translated_news",
                "websocket translated broadcast stream closed"
            );
            false
        }
    }
}

/// Traite les metadata live fallback compatibles avec le contrat edge.
pub(super) async fn handle_live_metadata_broadcast(
    sender: &mut SplitSink<WebSocket, Message>,
    live_metadata_message: Result<EdgeLiveMessage, RecvError>,
) -> bool {
    match live_metadata_message {
        Ok(message) => {
            let Some(payload) = serialize_live_metadata_message(message) else {
                return true;
            };

            send_text(sender, payload).await.is_ok()
        }
        Err(RecvError::Lagged(skipped)) => {
            warn!(
                skipped,
                stream = "live_metadata",
                "websocket subscriber lagged behind live metadata broadcast stream"
            );

            send_lagged(sender, skipped).await.is_ok()
        }
        Err(RecvError::Closed) => {
            info!(
                stream = "live_metadata",
                "websocket live metadata broadcast stream closed"
            );
            false
        }
    }
}
