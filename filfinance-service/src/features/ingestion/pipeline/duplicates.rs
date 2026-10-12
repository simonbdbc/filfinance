//! Logs agreges des doublons rejetes par le pipeline.

// `HashMap` agrege les compteurs par source et type d'evenement.
use std::collections::HashMap;

// `warn!` conserve un signal structure sans saturer les logs sur chaque doublon.
use tracing::warn;

/// Intervalle de flush des logs de doublons pour eviter un warning par evenement.
pub(super) const DUPLICATE_LOG_INTERVAL_SECS: u64 = 5;

/// Emet un log agrege par source et type d'evenement pour tous les doublons accumules.
pub(super) fn flush_duplicate_logs(pending_duplicate_counts: &mut HashMap<(String, String), u64>) {
    // `drain()` vide la map en meme temps qu'il l'itere, ce qui correspond exactement au besoin de flush.
    for ((source, event_kind), duplicates_dropped) in pending_duplicate_counts.drain() {
        warn!(
            source = %source,
            event_kind = %event_kind,
            duplicates_dropped,
            "duplicates dropped: {}",
            duplicates_dropped
        );
    }
}
