//! Worker periodique du collecteur FinancialJuice site.

mod periodic;
mod startup;

use reqwest::Client;
// Le signal partagé permet aux deux boucles source de quitter immédiatement pendant un redeploy.
use tokio::{sync::watch, task::JoinHandle};
// Les logs structurés rendent visible le démarrage du poll.
use tracing::info;

use crate::{app_state::AppState, config::FinancialJuiceSiteConfig};

use self::periodic::run_periodic_poll_loop_with_shutdown;
use super::super::{FINANCIAL_JUICE_MAIN_FEED_ID, FinancialJuiceSiteCollector};

/// Handles des deux boucles independantes du collecteur FinancialJuice.
pub(crate) struct FinancialJuiceWorkerHandles {
    /// Poll et backfill bornes par la configuration de source.
    pub(crate) poll_handle: JoinHandle<()>,
    /// Connexion SignalR live avec reconnexion bornee par son backoff.
    pub(crate) live_handle: JoinHandle<()>,
}

/// Lance le collecteur FinancialJuice primaire via session web authentifiée.
pub(crate) fn spawn_site(
    state: AppState,
    config: FinancialJuiceSiteConfig,
    client: Client,
    shutdown_rx: watch::Receiver<bool>,
) -> FinancialJuiceWorkerHandles {
    info!(
        source = %config.name,
        configured_enabled = config.enabled,
        home_url = %config.home_url,
        feed_id = FINANCIAL_JUICE_MAIN_FEED_ID,
        poll_interval_ms = config.poll_interval_ms,
        request_timeout_ms = config.request_timeout_ms,
        session_refresh_interval_ms = config.session_refresh_interval_ms,
        live_retry_backoff_ms = config.live_retry_backoff_ms,
        startup_backfill_page_limit = config.startup_backfill_page_limit,
        "financial juice site collector started"
    );

    // Le poll/backfill et le live partagent le même collecteur clonable et donc le même cache de
    // session, mais chaque boucle garde son propre receiver et son propre handle de shutdown.
    let collector = FinancialJuiceSiteCollector::new(config.clone(), client);
    let live_state = state.clone();
    let live_collector = collector.clone();
    let live_shutdown_rx = shutdown_rx.clone();
    let live_handle = tokio::spawn(async move {
        live_collector
            .run_live_loop(live_state, live_shutdown_rx)
            .await;
    });
    let poll_handle = tokio::spawn(async move {
        run_periodic_poll_loop_with_shutdown(state, config, collector, shutdown_rx).await;
    });

    FinancialJuiceWorkerHandles {
        poll_handle,
        live_handle,
    }
}
