//! Persistence durable d'un événement d'ingestion.

// Les logs d'erreur restent au plus près de la persistence.
use tracing::error;

use crate::{app_state::AppState, features::news::domain::NewsEvent};

use super::{broadcast::broadcast_persisted_event, trace};

/// Persiste un événement puis le diffuse si la base l'a accepté.
pub(super) async fn persist_then_broadcast(state: &AppState, event: &NewsEvent) {
    match state.news.insert_event(event).await {
        Ok(Some(stored)) => {
            // Cas nominal: la base a inséré l'événement et on récupère la ligne persistente.
            trace::emit_persisted(state, &stored);
            broadcast_persisted_event(state, &stored);
        }
        Ok(None) => {
            // Cas dedup durable: la mémoire l'avait laissé passer, mais SQLite connaissait déjà
            // la signature. C'est normal si le process a redémarré ou si la fenêtre mémoire a expiré.
            trace::emit_storage_deduplicated(state, event);
        }
        Err(error) => {
            // Une erreur de persistence est critique pour cet événement car la base fait foi.
            trace::emit_persist_failed(state, event, error.to_string());
            // Le log structuré garde l'erreur et la source, mais le pipeline continue pour
            // ne pas bloquer tout le service sur un seul item.
            error!(%error, source = %event.source, "failed to persist news event");
        }
    }
}
