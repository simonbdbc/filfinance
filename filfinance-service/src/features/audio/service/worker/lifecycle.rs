//! Cycle de vie du worker TTS serveur.

// `stream` porte la boucle de reception, `warmup` le prechauffage provider.
mod stream;
mod warmup;

// `Arc` partage l'état runtime mutable entre le worker TTS et les handlers HTTP.
use std::sync::Arc;

// `watch` apporte le signal de shutdown et `JoinHandle` permet au bootstrap d'attendre la tache.
use tokio::{
    sync::{Mutex, mpsc, watch},
    task::JoinHandle,
};
// Les logs signalent les workers optionnels désactivés.
use tracing::info;

use crate::{app_state::AppState, features::audio::tts::provider::TtsProvider};

/// Lance le worker TTS si la fonctionnalité est activée.
pub(crate) fn spawn_tts_worker(
    state: AppState,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Option<JoinHandle<()>> {
    if !state.config.tts.enabled {
        info!("tts worker disabled");
        return None;
    }

    if state.config.tts.worker_count > 1 {
        return spawn_tts_worker_pool(state, shutdown_rx);
    }

    let mut provider = TtsProvider::from_config(&state.config.tts, Arc::clone(&state.tts_runtime));

    Some(tokio::spawn(async move {
        warmup::warmup_provider(&state, &mut provider).await;
        stream::run_translation_stream(&state, &mut provider, &mut shutdown_rx).await;
        provider.shutdown().await;
    }))
}

fn spawn_tts_worker_pool(
    state: AppState,
    shutdown_rx: watch::Receiver<bool>,
) -> Option<JoinHandle<()>> {
    let worker_count = state.config.tts.worker_count;
    Some(tokio::spawn(async move {
        info!(worker_count, "tts worker pool starting");

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
                let mut provider = TtsProvider::from_config(
                    &worker_state.config.tts,
                    Arc::clone(&worker_state.tts_runtime),
                );
                warmup::warmup_provider(&worker_state, &mut provider).await;
                stream::run_translation_queue_stream(
                    worker_state,
                    &mut provider,
                    worker_rx,
                    &mut worker_shutdown_rx,
                    worker_index,
                    worker_index == 0,
                )
                .await;
                provider.shutdown().await;
            }));
        }

        if let Err(error) = dispatcher_handle.await {
            info!(%error, "tts dispatcher join failed");
        }

        for handle in worker_handles {
            if let Err(error) = handle.await {
                info!(%error, "tts pool worker join failed");
            }
        }
    }))
}
