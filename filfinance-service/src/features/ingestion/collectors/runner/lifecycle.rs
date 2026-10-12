//! Cycle de vie commun des collecteurs actifs.

// `Client` est partagé par les sources.
use reqwest::Client;
// Les handles permettent au bootstrap d'attendre les collecteurs pendant l'arret borne.
use tokio::{sync::watch, task::JoinHandle};

use crate::{app_state::AppState, config::SourcesConfig};

use super::super::{financial_juice, x};

/// Handles des collecteurs qui alimentent la file d'ingestion live.
pub(crate) struct CollectorWorkerHandles {
    /// Poll/backfill periodique de la source primaire FinancialJuice.
    pub(crate) financial_juice_poll_handle: JoinHandle<()>,
    /// Flux SignalR FinancialJuice, separe du poll pour ne jamais bloquer le rattrapage.
    pub(crate) financial_juice_live_handle: JoinHandle<()>,
    /// Polling officiel X dynamique.
    pub(crate) x_handle: JoinHandle<()>,
}

/// Instancie et lance tous les collecteurs connus du service.
pub(crate) fn spawn(
    state: AppState,
    client: &Client,
    shutdown_rx: watch::Receiver<bool>,
) -> CollectorWorkerHandles {
    let config: &SourcesConfig = &state.config.sources;
    let x_config = config.x.clone();
    // FinancialJuice site devient la source primaire. Il gère lui-même son bootstrap paginé et son live.
    let financial_juice_handles = financial_juice::spawn_site(
        state.clone(),
        config.financial_juice.clone(),
        client.clone(),
        shutdown_rx.clone(),
    );
    // Les anciens collecteurs hors scope sont retires: les sources actives restent explicites.
    let x_handle = x::spawn(state, x_config, client.clone(), shutdown_rx);

    CollectorWorkerHandles {
        financial_juice_poll_handle: financial_juice_handles.poll_handle,
        financial_juice_live_handle: financial_juice_handles.live_handle,
        x_handle,
    }
}
