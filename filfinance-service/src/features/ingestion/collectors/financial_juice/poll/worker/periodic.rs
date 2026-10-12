//! Boucle périodique du worker FinancialJuice site.

// Duration cadence le poll périodique et les backoffs amont.
use std::time::Duration;

// `watch` annule le cycle courant au shutdown; `interval` porte la cadence de rattrapage périodique.
use tokio::{
    sync::watch,
    time::{MissedTickBehavior, interval, sleep},
};
// Les logs structurés rendent visibles les réparations et incidents du poll.
use tracing::{info, warn};

use crate::{
    app_state::AppState, config::FinancialJuiceSiteConfig,
    features::ingestion::collectors::emit_items_to_pipeline,
};

use super::{
    super::super::{
        FinancialJuiceSiteCollector, handle_runtime_upstream_error,
        reset_runtime_rate_limit_failures,
    },
    startup::run_startup_backfill,
};

/// Lance le rattrapage periodique et abandonne cooperativement toute requete/queue en cours.
pub(super) async fn run_periodic_poll_loop_with_shutdown(
    state: AppState,
    config: FinancialJuiceSiteConfig,
    collector: FinancialJuiceSiteCollector,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    if *shutdown_rx.borrow() {
        info!(source = %config.name, "financial juice periodic poll skipped after shutdown");
        return;
    }
    let source_name = config.name.clone();

    tokio::select! {
        changed = shutdown_rx.changed() => {
            if changed.is_err() || *shutdown_rx.borrow() {
                info!(source = %source_name, "financial juice periodic poll received shutdown signal");
            }
        }
        _ = run_periodic_poll_loop_until_shutdown(state, config, collector) => {}
    }
}

/// Boucle effective, separee pour que le `select!` externe puisse annuler I/O, backoff et queue.
async fn run_periodic_poll_loop_until_shutdown(
    state: AppState,
    config: FinancialJuiceSiteConfig,
    collector: FinancialJuiceSiteCollector,
) {
    // `interval` fournit une cadence stable et `MissedTickBehavior::Delay` évite les rafales
    // si le process a été bloqué plus longtemps qu'un tick.
    let mut ticker = interval(Duration::from_millis(config.poll_interval_ms));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // On consomme le premier tick immédiat pour éviter un second poll instantané juste après le backfill.
    ticker.tick().await;
    let mut was_enabled = state.financial_juice_runtime.is_enabled();
    if was_enabled {
        info!(source = %config.name, "financial juice site runtime enabled");
        if !run_startup_backfill(&state, &config, &collector).await {
            return;
        }
    }

    loop {
        ticker.tick().await;

        if !state.financial_juice_runtime.is_enabled() {
            if was_enabled {
                info!(source = %config.name, "financial juice site runtime disabled");
            }
            was_enabled = false;
            continue;
        }

        if !was_enabled {
            info!(source = %config.name, "financial juice site runtime enabled");
            was_enabled = true;
            if !run_startup_backfill(&state, &config, &collector).await {
                return;
            }
        }

        match collector.poll_latest().await {
            Ok(items) => {
                reset_runtime_rate_limit_failures(&state);
                if let Err(error) = state.news.repair_financial_juice_events(&items).await {
                    warn!(
                        %error,
                        source = %config.name,
                        "financial juice periodic repair failed"
                    );
                }
                if !emit_items_to_pipeline(&config.name, items, &state.ingest_tx).await {
                    return;
                }
            }
            Err(error) => {
                let action =
                    handle_runtime_upstream_error(&state, &config.name, "periodic_catchup", &error);
                if action.primary_disabled() {
                    was_enabled = false;
                }
                warn!(
                    %error,
                    source = %config.name,
                    "financial juice periodic poll failed"
                );
                sleep(Duration::from_millis(config.rate_limit_backoff_ms)).await;
            }
        }
    }
}
