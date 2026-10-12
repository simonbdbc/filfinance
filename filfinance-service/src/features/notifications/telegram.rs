//! Worker de relay Telegram abonne au flux broadcast interne.

mod bridge;
mod client;
mod format;
mod sender;

// reqwest::Client execute les appels HTTP vers Telegram.
use reqwest::Client;
// `mpsc` protege le downstream Telegram via une queue bornee; `watch` propage l'arret process.
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
// info/warn structurent l'observabilite du downstream Telegram.
use tracing::{info, warn};

use crate::{app_state::AppState, features::translation::domain::PersistedNewsTranslation};

use bridge::spawn_telegram_broadcast_bridge;
#[allow(unused_imports)]
pub(in crate::features::notifications) use client::{TelegramSendError, execute_send_with_retry};
pub(in crate::features::notifications) use format::build_telegram_message;
use sender::spawn_telegram_sender;

/// Handles des deux etapes Telegram, gardees hors du hot path live.
pub(crate) struct TelegramWorkerHandles {
    /// Pont broadcast -> queue bornee Telegram.
    pub(crate) bridge_handle: JoinHandle<()>,
    /// Consommateur de la queue et appels HTTP Telegram.
    pub(crate) sender_handle: JoinHandle<()>,
}

/// Lance le consommateur Telegram si la fonctionnalite est activee.
pub(crate) fn spawn(
    state: AppState,
    http_client: Client,
    shutdown_rx: watch::Receiver<bool>,
) -> Option<TelegramWorkerHandles> {
    if !state.config.telegram.enabled {
        info!("telegram relay disabled");
        return None;
    }

    if state.config.telegram.bot_token.is_none() || state.config.telegram.chat_ids.is_empty() {
        warn!("telegram relay enabled but token/chat ids are missing");
        return None;
    }

    // La queue bornee isole les delais Telegram du flux traduit principal.
    // Sans cette etape, une API Telegram lente pourrait ralentir la traduction et le WebSocket.
    let (queue_tx, queue_rx) =
        mpsc::channel::<PersistedNewsTranslation>(state.config.telegram.queue_buffer);

    let bridge_handle =
        spawn_telegram_broadcast_bridge(state.clone(), queue_tx, shutdown_rx.clone());
    let sender_handle = spawn_telegram_sender(state, http_client, queue_rx, shutdown_rx);

    Some(TelegramWorkerHandles {
        bridge_handle,
        sender_handle,
    })
}
