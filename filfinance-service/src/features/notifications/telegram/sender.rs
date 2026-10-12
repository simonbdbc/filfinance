//! Consommation de la queue Telegram et orchestration d'un message par chat.

mod delivery;
mod trace;

// reqwest::Client execute les appels HTTP vers Telegram.
use reqwest::Client;
// `mpsc` recoit les traductions a envoyer, `watch` annule les retries et `Instant` garde le rate
// limit local.
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::Instant,
};
use tracing::{error, info};

use crate::{app_state::AppState, features::translation::domain::PersistedNewsTranslation};

use self::delivery::send_news_item;

/// Vide la queue Telegram et applique le budget de debit / retry propre a ce downstream.
pub(super) fn spawn_telegram_sender(
    state: AppState,
    http_client: Client,
    queue_rx: mpsc::Receiver<PersistedNewsTranslation>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if *shutdown_rx.borrow() {
            info!("telegram sender skipped after shutdown");
            return;
        }

        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    info!("telegram sender received shutdown signal");
                }
            }
            _ = run_telegram_sender(state, http_client, queue_rx) => {}
        }
    })
}

/// Boucle effective de consommation; le wrapper annule en cours les retries non critiques.
async fn run_telegram_sender(
    state: AppState,
    http_client: Client,
    mut queue_rx: mpsc::Receiver<PersistedNewsTranslation>,
) {
    // On garde le dernier envoi pour appliquer un debit minimum entre appels HTTP.
    let mut last_http_send_at: Option<Instant> = None;

    while let Some(item) = queue_rx.recv().await {
        if let Err(error) =
            send_news_item(&http_client, &state, &item, &mut last_http_send_at).await
        {
            error!(%error, "failed to relay news to telegram");
        }
    }

    info!("telegram sender worker stopped because queue is closed");
}
