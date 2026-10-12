//! Monitoring leger du fallback navigateur FinancialJuice.

use std::time::Duration;

use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{MissedTickBehavior, interval},
};
use tracing::{info, warn};

use crate::app_state::AppState;

use super::runtime::browser_collector_status_stale_after_ms;

/// Lance une boucle qui journalise explicitement l'etat du fallback navigateur quand il doit servir.
pub(crate) fn spawn_browser_fallback_monitor(
    state: AppState,
    mut shutdown_rx: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let poll_interval_ms = state
            .config
            .sources
            .financial_juice_browser
            .source
            .poll_interval_ms;
        let stale_after_ms = browser_collector_status_stale_after_ms(poll_interval_ms);
        let mut ticker = interval(Duration::from_millis(poll_interval_ms.max(5_000)));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut last_problem_key: Option<String> = None;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let primary_enabled = state.financial_juice_runtime.is_enabled();
                    let status = state.financial_juice_browser_runtime.effective_status(
                        primary_enabled,
                        stale_after_ms,
                    );
                    let source = &state.config.sources.financial_juice_browser.source.name;

                    if !status.fallback_active {
                        last_problem_key = None;
                        continue;
                    }

                    if status.enabled {
                        if last_problem_key.take().is_some() {
                            info!(
                                source = %source,
                                status = %status.status,
                                service_accessible = status.service_accessible,
                                main_feed_available = status.main_feed_available,
                                message = %status.message.as_deref().unwrap_or(""),
                                "financial juice browser fallback is usable"
                            );
                        }
                        continue;
                    }

                    let problem_key = format!(
                        "{}|{}|{}|{}|{}",
                        status.status,
                        status.message.as_deref().unwrap_or(""),
                        status.updated_at.as_deref().unwrap_or(""),
                        status.service_accessible,
                        status.main_feed_available
                    );
                    if last_problem_key.as_deref() == Some(problem_key.as_str()) {
                        continue;
                    }

                    warn!(
                        source = %source,
                        status = %status.status,
                        service_accessible = status.service_accessible,
                        main_feed_available = status.main_feed_available,
                        stale = status.stale,
                        message = %status.message.as_deref().unwrap_or(""),
                        "financial juice browser fallback is not usable"
                    );
                    last_problem_key = Some(problem_key);
                }
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        break;
                    }
                }
            }
        }
    })
}
