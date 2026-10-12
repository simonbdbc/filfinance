//! Boucle de reconnexion du live SignalR FinancialJuice.

// `Duration` cadence la reconnexion live après incident.
use std::time::Duration;

// `watch` annule un connect/backoff SignalR en cours; `sleep` porte le backoff de reconnexion.
use tokio::{sync::watch, time::sleep};
// Les logs structurés rendent lisibles les phases live et les reconnexions.
use tracing::{info, warn};

use crate::app_state::AppState;

use super::super::{
    FinancialJuiceSiteCollector, handle_runtime_upstream_error, reset_runtime_rate_limit_failures,
};

impl FinancialJuiceSiteCollector {
    /// Boucle de live SignalR: négociation, WebSocket, décodage `sendUpdates`, reconnexion.
    pub(in crate::features::ingestion::collectors::financial_juice) async fn run_live_loop(
        self,
        state: AppState,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        if *shutdown_rx.borrow() {
            info!(source = %self.config.name, "financial juice live skipped after shutdown");
            return;
        }
        let source_name = self.config.name.clone();

        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    info!(source = %source_name, "financial juice live received shutdown signal");
                }
            }
            _ = self.run_live_loop_until_shutdown(state) => {}
        }
    }

    /// Boucle SignalR effective, annulee par le wrapper des que le processus s'arrete.
    async fn run_live_loop_until_shutdown(self, state: AppState) {
        info!(
            source = %self.config.name,
            live_retry_backoff_ms = self.config.live_retry_backoff_ms,
            "financial juice live signalr started"
        );

        let mut logged_runtime_disabled = false;
        loop {
            if !state.financial_juice_runtime.is_enabled() {
                if !logged_runtime_disabled {
                    info!(source = %self.config.name, "financial juice live runtime disabled");
                    logged_runtime_disabled = true;
                }
                sleep(Duration::from_millis(self.config.live_retry_backoff_ms)).await;
                continue;
            }

            logged_runtime_disabled = false;
            let mut retry_backoff_ms = self.config.live_retry_backoff_ms;
            match self.run_live_once(&state).await {
                Ok(()) => {
                    reset_runtime_rate_limit_failures(&state);
                    warn!(
                        source = %self.config.name,
                        retry_backoff_ms,
                        "financial juice live loop exited unexpectedly, reconnecting"
                    );
                }
                Err(error) => {
                    let action = handle_runtime_upstream_error(
                        &state,
                        &self.config.name,
                        "live_signalr",
                        &error,
                    );
                    if action.use_rate_limit_backoff() {
                        retry_backoff_ms = self.config.rate_limit_backoff_ms;
                    }
                    warn!(
                        %error,
                        source = %self.config.name,
                        retry_backoff_ms,
                        "financial juice live signalr failed"
                    );
                }
            }

            self.invalidate_bootstrap().await;
            sleep(Duration::from_millis(retry_backoff_ms)).await;
        }
    }
}
