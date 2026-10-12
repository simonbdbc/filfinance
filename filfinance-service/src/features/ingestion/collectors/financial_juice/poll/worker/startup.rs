//! Backfill initial du worker FinancialJuice site.

// Les logs structurés rendent visibles les réparations et incidents du backfill.
use tracing::warn;

use crate::{
    app_state::AppState, config::FinancialJuiceSiteConfig,
    features::ingestion::collectors::emit_items_to_pipeline,
};

use super::super::super::{
    FinancialJuiceSiteCollector, handle_runtime_upstream_error, reset_runtime_rate_limit_failures,
};

/// Execute le backfill J/J-1 avant la boucle périodique.
pub(super) async fn run_startup_backfill(
    state: &AppState,
    config: &FinancialJuiceSiteConfig,
    collector: &FinancialJuiceSiteCollector,
) -> bool {
    // On remplit d'abord l'historique J/J-1 avant d'entrer dans le rythme de poll périodique.
    match collector.fetch_startup_backfill().await {
        Ok(items) => {
            reset_runtime_rate_limit_failures(state);
            if let Err(error) = state.news.repair_financial_juice_events(&items).await {
                warn!(
                    %error,
                    source = %config.name,
                    "financial juice startup repair failed"
                );
            }
            emit_items_to_pipeline(&config.name, items, &state.ingest_tx).await
        }
        Err(error) => {
            handle_runtime_upstream_error(state, &config.name, "startup_backfill", &error);
            warn!(
                %error,
                source = %config.name,
                "financial juice startup backfill failed"
            );
            true
        }
    }
}
