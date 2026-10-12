//! Emission vers la queue canonique d'ingestion.

// `mpsc` pousse les événements normalisés dans la queue d'ingestion canonique.
use tokio::sync::mpsc;
// Logs structurés pour les arrêts de queue.
use tracing::error;

use crate::features::news::domain::NewsEvent;

/// Emission unifiée vers la queue d'ingestion pour le bootstrap et le polling périodique.
pub(crate) async fn emit_items_to_pipeline(
    source_name: &str,
    items: Vec<NewsEvent>,
    ingest_tx: &mpsc::Sender<NewsEvent>,
) -> bool {
    for mut item in items {
        // La signature est toujours recalculée juste avant d'entrer dans le hot path.
        item.recompute_signature();

        // Si la queue d'ingestion est fermée, le service est en train de s'arrêter.
        if ingest_tx.send(item).await.is_err() {
            error!(source = %source_name, "ingest channel closed");
            return false;
        }
    }

    true
}
