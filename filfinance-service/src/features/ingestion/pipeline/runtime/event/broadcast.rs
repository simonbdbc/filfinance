//! Broadcast d'un événement persisté vers les downstreams internes.

use crate::{app_state::AppState, features::news::domain::PersistedNewsEvent};

/// Diffuse un événement brut persiste vers le worker de traduction.
pub(super) fn broadcast_persisted_event(state: &AppState, stored: &PersistedNewsEvent) {
    // Le `broadcast` brut alimente ensuite le worker de traduction.
    match state.broadcast_tx.send(stored.clone()) {
        Ok(subscriber_count) => {
            // Le nombre de receveurs est utile au debug du fanout temps réel.
            state.emit_test_trace_for_event(
                stored,
                "pipeline_broadcast_emitted",
                "ok",
                "Evenement brut emis vers le worker de traduction.",
                Some(format!("subscriber_count={subscriber_count}")),
            );
        }
        Err(_) => {
            // Ici l'event a bien été persisté, mais aucun consommateur n'était abonné.
            state.emit_test_trace_for_event(
                stored,
                "pipeline_broadcast_emitted",
                "warning",
                "Evenement brut emis sans consommateur actif.",
                None,
            );
        }
    }
}
