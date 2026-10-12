//! Pont entre le broadcast traduit et la queue Telegram bornee.

// `broadcast` relaye les traductions temps reel, `mpsc` pousse vers le sender et `watch` arrete les
// deux boucles Telegram sans affecter le broadcast live.
use tokio::{
    sync::{broadcast::error::RecvError, mpsc, watch},
    task::JoinHandle,
};
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::{news::domain::PersistedNewsEvent, translation::domain::PersistedNewsTranslation},
};

/// Consomme le broadcast traduit temps reel et tente de pousser les evenements vers la queue Telegram.
pub(super) fn spawn_telegram_broadcast_bridge(
    state: AppState,
    queue_tx: mpsc::Sender<PersistedNewsTranslation>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if *shutdown_rx.borrow() {
            info!("telegram broadcast bridge skipped after shutdown");
            return;
        }

        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    info!("telegram broadcast bridge received shutdown signal");
                }
            }
            _ = run_telegram_broadcast_bridge(state, queue_tx) => {}
        }
    })
}

/// Boucle effective du pont; le wrapper l'annule pendant un shutdown process.
async fn run_telegram_broadcast_bridge(
    state: AppState,
    queue_tx: mpsc::Sender<PersistedNewsTranslation>,
) {
    // Chaque worker Telegram prend son propre abonnement broadcast.
    let mut rx = state.translated_broadcast_tx.subscribe();

    loop {
        match rx.recv().await {
            Ok(item) => {
                // On recharge l'event source pour pouvoir attacher les traces E2E au bon run de test.
                let source_event = load_source_event(&state, item.news_event_id).await;

                // En cas de burst, on prefere dropper le downstream Telegram plutot que ralentir le service.
                if queue_tx.try_send(item).is_err() {
                    if let Some(source_event) = source_event.as_ref() {
                        state.emit_test_trace_for_event(
                            source_event,
                            "telegram_queue_dropped",
                            "warning",
                            "Le downstream Telegram a droppe l'item car sa queue est pleine.",
                            None,
                        );
                    }
                    warn!("telegram queue is full, dropping realtime item");
                } else if let Some(source_event) = source_event.as_ref() {
                    state.emit_test_trace_for_event(
                        source_event,
                        "telegram_queue_enqueued",
                        "ok",
                        "La traduction a ete placee dans la queue Telegram.",
                        None,
                    );
                }
            }
            Err(RecvError::Lagged(skipped)) => {
                // `Lagged` signifie que ce receveur etait trop lent par rapport au `broadcast`.
                warn!(
                    skipped,
                    "telegram relay lagged behind translated broadcast stream"
                );
            }
            Err(RecvError::Closed) => {
                info!("telegram relay stopped because translated broadcast stream is closed");
                break;
            }
        }
    }
}

/// Recharge l'evenement source uniquement pour les besoins de trace du downstream Telegram.
async fn load_source_event(state: &AppState, news_event_id: i64) -> Option<PersistedNewsEvent> {
    match state.news.get_news_by_id(news_event_id).await {
        Ok(source_event) => source_event,
        Err(error) => {
            // Une absence de source ne doit pas casser Telegram; on degrade simplement la trace.
            warn!(
                %error,
                news_event_id,
                "failed to load source event for telegram trace"
            );
            None
        }
    }
}
