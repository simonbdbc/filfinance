//! Replay initial des traductions récentes sur reconnect WebSocket.

use axum::extract::ws::{Message, WebSocket};
// `SplitSink` est la moitié "écriture" retournée par `socket.split()`.
use futures_util::stream::SplitSink;
// Les logs structurés gardent les erreurs de replay visibles.
use tracing::warn;

use crate::app_state::AppState;

use super::{emit::emit_live_item, payload::build_live_news_item};
use crate::features::live::websocket::TRANSLATION_REPLAY_LIMIT;

/// Rejoue un petit backlog de traductions prêtes pour réduire les trous au reconnect client.
pub(in crate::features::live::websocket::news) async fn replay_recent_translations(
    sender: &mut SplitSink<WebSocket, Message>,
    state: &AppState,
) -> Result<(), ()> {
    // Le replay couvre seulement les derniers items FR prêts pour limiter le coût d'un reconnect.
    let items = match state
        .news
        .list_recent_translations(
            &state.config.translation.target_language,
            TRANSLATION_REPLAY_LIMIT,
        )
        .await
    {
        Ok(items) => items,
        Err(error) => {
            warn!(%error, "failed to load recent translations for websocket replay");
            Vec::new()
        }
    };

    for item in items {
        // On reconstruit le `LiveNewsItem` avec le même helper que le live, pour garder un contrat unique.
        let live_item = build_live_news_item(state, item).await;
        emit_live_item(
            sender,
            state,
            live_item,
            "user_ws_replay_emitted",
            "Un item traduit recent a ete rejoue a un client WebSocket user-facing.",
        )
        .await?;
    }

    Ok(())
}
