//! Boucle de vie du worker edge metadata.

// Le worker edge s'abonne au broadcast des traductions et écoute le shutdown applicatif.
// `JoinHandle` permet à `main` d'attendre proprement la tâche au shutdown.
use tokio::{
    sync::{broadcast::error::RecvError, watch},
    task::JoinHandle,
    time::{Duration, MissedTickBehavior, interval},
};
// Les logs sont le signal d'observabilité principal de ce downstream optionnel.
use tracing::{info, warn};

use crate::app_state::AppState;

use super::{
    super::publisher::EdgePublisher,
    publish::{publish_source_status, publish_translation},
};

/// Cadence de publication de l'etat public des sources vers le control plane edge.
const SOURCE_STATUS_PUBLISH_INTERVAL_SECS: u64 = 20;

/// Lance le bridge des producteurs vers le point d'entree live commun.
pub fn spawn(state: AppState, mut shutdown_rx: watch::Receiver<bool>) -> Option<JoinHandle<()>> {
    if !state.config.edge.publish_enabled && !state.config.live_fanout.enabled {
        info!("live metadata publisher and Redis fan-out disabled");
        return None;
    }

    let publisher = if state.config.edge.publish_enabled {
        match EdgePublisher::from_config(&state.config.edge) {
            Ok(publisher) => Some(publisher),
            Err(error) => {
                warn!(%error, "edge metadata publisher failed to initialize");
                None
            }
        }
    } else {
        None
    };

    Some(tokio::spawn(async move {
        // Chaque downstream prend son propre abonnement, sans perturber WebSocket/TTS/Telegram.
        let mut rx = state.translated_broadcast_tx.subscribe();
        let mut source_status_ticker =
            interval(Duration::from_secs(SOURCE_STATUS_PUBLISH_INTERVAL_SECS));
        source_status_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        publish_source_status(&state, publisher.as_ref()).await;
        source_status_ticker.tick().await;

        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    match changed {
                        Ok(()) if *shutdown_rx.borrow() => {
                            info!("edge metadata publisher received shutdown signal");
                            break;
                        }
                        Ok(()) => {
                            // Le signal reste false: continuer à diffuser normalement.
                        }
                        Err(_) => {
                            info!("edge metadata publisher shutdown channel closed");
                            break;
                        }
                    }
                }
                received = rx.recv() => match received {
                    Ok(item) => publish_translation(&state, publisher.as_ref(), item).await,
                    Err(RecvError::Lagged(skipped)) => {
                        warn!(skipped, "edge metadata publisher lagged behind translated stream");
                    }
                    Err(RecvError::Closed) => {
                        info!("edge metadata publisher stopped because translated stream is closed");
                        break;
                    }
                },
                _ = source_status_ticker.tick() => {
                    publish_source_status(&state, publisher.as_ref()).await;
                }
            }
        }
    }))
}
