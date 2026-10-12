//! Boucle de consommation du broadcast brut par le worker de traduction.

// `watch` sert au signal de shutdown gracieux, `mpsc` a la queue partagee du mode pool.
use std::sync::Arc;

use tokio::sync::{Mutex, broadcast::error::RecvError, mpsc, watch};
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::{news::domain::PersistedNewsEvent, translation::provider::TranslationProvider},
};

use super::super::processing::translate_and_store_item;

/// Consomme le broadcast brut jusqu'au shutdown ou a la fermeture du canal.
pub(super) async fn run_translation_loop(
    state: AppState,
    mut provider: TranslationProvider,
    shutdown_rx: &mut watch::Receiver<bool>,
) {
    let mut rx = state.broadcast_tx.subscribe();

    loop {
        // Le worker ecoute a la fois le signal d'arret et le flux de news brutes.
        tokio::select! {
            changed = shutdown_rx.changed() => {
                match changed {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!("translation worker received shutdown signal");
                        break;
                    }
                    Ok(()) => {
                        // Le signal reste false: continuer a ecouter normalement.
                    }
                    Err(_) => {
                        info!("translation worker shutdown channel closed");
                        break;
                    }
                }
            }
            received = rx.recv() => match received {
                Ok(item) => {
                    translate_and_store_item(&state, &mut provider, &item, true).await;
                }
                Err(RecvError::Lagged(skipped)) => {
                    // Un worker trop lent perd des messages; on le logue sans arreter le process.
                    warn!(
                        skipped,
                        "translation worker lagged behind news broadcast stream"
                    );
                }
                Err(RecvError::Closed) => {
                    info!("translation worker stopped because news broadcast stream is closed");
                    break;
                }
            }
        }
    }

    provider.shutdown().await;
}

/// Reçoit chaque news brute une seule fois et l'enfile pour le pool de traduction.
pub(super) async fn run_translation_dispatcher(
    state: AppState,
    queue_tx: mpsc::Sender<PersistedNewsEvent>,
    shutdown_rx: &mut watch::Receiver<bool>,
) {
    let mut rx = state.broadcast_tx.subscribe();

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                match changed {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!("translation dispatcher received shutdown signal");
                        break;
                    }
                    Ok(()) => {}
                    Err(_) => {
                        info!("translation dispatcher shutdown channel closed");
                        break;
                    }
                }
            }
            received = rx.recv() => match received {
                Ok(item) => {
                    if queue_tx.send(item).await.is_err() {
                        info!("translation dispatcher stopped because worker queue is closed");
                        break;
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    warn!(
                        skipped,
                        "translation dispatcher lagged behind news broadcast stream"
                    );
                }
                Err(RecvError::Closed) => {
                    info!("translation dispatcher stopped because news broadcast stream is closed");
                    break;
                }
            }
        }
    }
}

/// Consomme la queue partagee par un worker du pool de traduction.
pub(super) async fn run_translation_queue_loop(
    state: AppState,
    mut provider: TranslationProvider,
    shared_rx: Arc<Mutex<mpsc::Receiver<PersistedNewsEvent>>>,
    shutdown_rx: &mut watch::Receiver<bool>,
    worker_index: usize,
) {
    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                match changed {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!(worker_index, "translation pool worker received shutdown signal");
                        break;
                    }
                    Ok(()) => {}
                    Err(_) => {
                        info!(worker_index, "translation pool worker shutdown channel closed");
                        break;
                    }
                }
            }
            received = receive_queued_news_event(&shared_rx) => match received {
                Some(item) => {
                    translate_and_store_item(&state, &mut provider, &item, true).await;
                }
                None => {
                    info!(worker_index, "translation pool worker stopped because queue is closed");
                    break;
                }
            }
        }
    }

    provider.shutdown().await;
}

async fn receive_queued_news_event(
    shared_rx: &Arc<Mutex<mpsc::Receiver<PersistedNewsEvent>>>,
) -> Option<PersistedNewsEvent> {
    let mut rx = shared_rx.lock().await;
    rx.recv().await
}
