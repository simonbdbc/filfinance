//! Boucle de reception du worker TTS.

// `broadcast` reçoit les traductions prêtes; `mpsc` partage le travail du mode pool.
use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{Mutex, broadcast::error::RecvError, mpsc, watch},
    time::{self, MissedTickBehavior},
};
// Les logs suivent les lags et fermetures de flux sans arrêter le service entier.
use tracing::{info, warn};

use crate::{
    app_state::AppState,
    features::{audio::tts::provider::TtsProvider, translation::domain::PersistedNewsTranslation},
};

use super::super::{replay_backfill::backfill_daily_replay_audio, translation::handle_translation};

const REPLAY_AUDIO_BACKFILL_INTERVAL_SECS: u64 = 60;

/// Consomme les traductions prêtes jusqu'au shutdown.
pub(super) async fn run_translation_stream(
    state: &AppState,
    provider: &mut TtsProvider,
    shutdown_rx: &mut watch::Receiver<bool>,
) {
    let mut translated_rx = state.translated_broadcast_tx.subscribe();
    let mut replay_backfill_interval =
        time::interval(Duration::from_secs(REPLAY_AUDIO_BACKFILL_INTERVAL_SECS));
    replay_backfill_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    replay_backfill_interval.tick().await;

    backfill_daily_replay_audio(state, provider).await;

    loop {
        // Le worker écoute l'arrêt gracieux et le flux de traductions prêtes.
        tokio::select! {
            changed = shutdown_rx.changed() => {
                match changed {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!("tts worker received shutdown signal");
                        break;
                    }
                    Ok(()) => {}
                    Err(_) => {
                        info!("tts worker shutdown channel closed");
                        break;
                    }
                }
            },
            received = translated_rx.recv() => match received {
                Ok(translation) => {
                    if should_process_translation(state, &translation) {
                        handle_translation(state, provider, translation).await;
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    warn!(skipped, "tts worker lagged behind translated broadcast stream");
                }
                Err(RecvError::Closed) => {
                    info!("tts worker stopped because translated broadcast stream is closed");
                    break;
                }
            },
            _ = replay_backfill_interval.tick() => {
                backfill_daily_replay_audio(state, provider).await;
            }
        }
    }
}

/// Reçoit chaque traduction prête une seule fois et l'enfile pour le pool TTS.
pub(super) async fn run_translation_dispatcher(
    state: AppState,
    queue_tx: mpsc::Sender<PersistedNewsTranslation>,
    shutdown_rx: &mut watch::Receiver<bool>,
) {
    let mut translated_rx = state.translated_broadcast_tx.subscribe();

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                match changed {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!("tts dispatcher received shutdown signal");
                        break;
                    }
                    Ok(()) => {}
                    Err(_) => {
                        info!("tts dispatcher shutdown channel closed");
                        break;
                    }
                }
            },
            received = translated_rx.recv() => match received {
                Ok(translation) => {
                    if should_process_translation(&state, &translation)
                        && queue_tx.send(translation).await.is_err()
                    {
                        info!("tts dispatcher stopped because worker queue is closed");
                        break;
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    warn!(skipped, "tts dispatcher lagged behind translated broadcast stream");
                }
                Err(RecvError::Closed) => {
                    info!("tts dispatcher stopped because translated broadcast stream is closed");
                    break;
                }
            }
        }
    }
}

/// Consomme la queue partagee par un worker du pool TTS.
pub(super) async fn run_translation_queue_stream(
    state: AppState,
    provider: &mut TtsProvider,
    shared_rx: Arc<Mutex<mpsc::Receiver<PersistedNewsTranslation>>>,
    shutdown_rx: &mut watch::Receiver<bool>,
    worker_index: usize,
    replay_backfill_enabled: bool,
) {
    let mut replay_backfill_interval =
        time::interval(Duration::from_secs(REPLAY_AUDIO_BACKFILL_INTERVAL_SECS));
    replay_backfill_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    replay_backfill_interval.tick().await;

    if replay_backfill_enabled {
        backfill_daily_replay_audio(&state, provider).await;
    }

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                match changed {
                    Ok(()) if *shutdown_rx.borrow() => {
                        info!(worker_index, "tts pool worker received shutdown signal");
                        break;
                    }
                    Ok(()) => {}
                    Err(_) => {
                        info!(worker_index, "tts pool worker shutdown channel closed");
                        break;
                    }
                }
            },
            received = receive_queued_translation(&shared_rx) => match received {
                Some(translation) => {
                    handle_translation(&state, provider, translation).await;
                }
                None => {
                    info!(worker_index, "tts pool worker stopped because queue is closed");
                    break;
                }
            },
            _ = replay_backfill_interval.tick(), if replay_backfill_enabled => {
                backfill_daily_replay_audio(&state, provider).await;
            }
        }
    }
}

/// Filtre les traductions qui peuvent produire un audio live.
fn should_process_translation(state: &AppState, translation: &PersistedNewsTranslation) -> bool {
    // Le worker TTS ne lit que la langue cible finale.
    if translation.language != state.config.translation.target_language {
        return false;
    }

    state.tts_runtime.record_received();

    // Les traductions en erreur ou incomplètes ne doivent pas produire d'audio parasite.
    if translation.status != "ready" {
        state.tts_runtime.record_skipped();
        return false;
    }

    if translation.narration_text.trim().is_empty() {
        state.tts_runtime.record_skipped();
        return false;
    }

    true
}

async fn receive_queued_translation(
    shared_rx: &Arc<Mutex<mpsc::Receiver<PersistedNewsTranslation>>>,
) -> Option<PersistedNewsTranslation> {
    let mut rx = shared_rx.lock().await;
    rx.recv().await
}
