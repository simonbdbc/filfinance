//! Lifecycle du worker de traduction.

// `backfill`, `stream` et `warmup` isolent les etapes du cycle de vie du worker.
mod backfill;
mod stream;
mod warmup;

// `watch` sert au signal de shutdown gracieux, `JoinHandle` au suivi par le bootstrap.
use std::sync::Arc;

use tokio::{
    sync::{Mutex, mpsc, watch},
    task::JoinHandle,
};
use tracing::info;

use crate::{app_state::AppState, features::translation::provider::TranslationProvider};

/// Lance le consommateur de traduction si la fonctionnalite est activee.
pub(crate) fn spawn(
    state: AppState,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Option<JoinHandle<()>> {
    if !state.config.translation.enabled {
        info!("translation worker disabled");
        return None;
    }

    if state.config.translation.worker_count > 1 {
        return spawn_pool(state, shutdown_rx);
    }

    let mut provider = TranslationProvider::from_config(&state.config.translation);

    Some(tokio::spawn(async move {
        warmup::warmup_provider(&state, &mut provider).await;
        backfill::run_startup_backfill(&state, &mut provider).await;
        stream::run_translation_loop(state, provider, &mut shutdown_rx).await;
    }))
}

fn spawn_pool(state: AppState, shutdown_rx: watch::Receiver<bool>) -> Option<JoinHandle<()>> {
    let worker_count = state.config.translation.worker_count;
    Some(tokio::spawn(async move {
        info!(worker_count, "translation worker pool starting");

        let mut startup_provider = TranslationProvider::from_config(&state.config.translation);
        warmup::warmup_provider(&state, &mut startup_provider).await;
        backfill::run_startup_backfill(&state, &mut startup_provider).await;
        startup_provider.shutdown().await;

        let (queue_tx, queue_rx) = mpsc::channel(state.config.runtime.broadcast_buffer);
        let shared_rx = Arc::new(Mutex::new(queue_rx));
        let mut dispatcher_shutdown_rx = shutdown_rx.clone();
        let dispatcher_state = state.clone();
        let dispatcher_handle = tokio::spawn(async move {
            stream::run_translation_dispatcher(
                dispatcher_state,
                queue_tx,
                &mut dispatcher_shutdown_rx,
            )
            .await;
        });

        let mut worker_handles = Vec::with_capacity(worker_count);
        for worker_index in 0..worker_count {
            let worker_state = state.clone();
            let worker_rx = Arc::clone(&shared_rx);
            let mut worker_shutdown_rx = shutdown_rx.clone();
            worker_handles.push(tokio::spawn(async move {
                let mut provider =
                    TranslationProvider::from_config(&worker_state.config.translation);
                warmup::warmup_provider(&worker_state, &mut provider).await;
                stream::run_translation_queue_loop(
                    worker_state,
                    provider,
                    worker_rx,
                    &mut worker_shutdown_rx,
                    worker_index,
                )
                .await;
            }));
        }

        if let Err(error) = dispatcher_handle.await {
            info!(%error, "translation dispatcher join failed");
        }

        for handle in worker_handles {
            if let Err(error) = handle.await {
                info!(%error, "translation pool worker join failed");
            }
        }
    }))
}
