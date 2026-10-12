//! Deduplication mémoire locale d'un événement en cours de pipeline.

// `HashMap`, `Duration` et `Instant` portent l'état de dedup mémoire du hot path.
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::features::news::domain::NewsEvent;

/// Indique si une signature vient déjà d'être vue dans la fenêtre TTL.
pub(super) fn is_recent_duplicate(
    recently_seen: &HashMap<String, Instant>,
    signature: &str,
    ttl: Duration,
) -> bool {
    recently_seen
        // `get(...)` retourne un `Option<&Instant>`.
        .get(signature)
        // `is_some_and(...)` dit "la valeur existe ET vérifie la condition".
        .is_some_and(|instant| instant.elapsed() < ttl)
}

/// Mémorise une signature acceptée dans la fenêtre de déduplication mémoire.
pub(super) fn remember_signature(recently_seen: &mut HashMap<String, Instant>, signature: &str) {
    // L'instant courant suffit: la purge TTL est faite avant chaque admission.
    recently_seen.insert(signature.to_owned(), Instant::now());
}

/// Agrège un doublon rejeté pour le prochain log périodique.
pub(super) fn record_pending_duplicate(
    pending_duplicate_counts: &mut HashMap<(String, String), u64>,
    event: &NewsEvent,
) {
    // `entry(...).or_default()` crée le compteur à `0` si besoin avant incrément.
    *pending_duplicate_counts
        .entry((event.source.clone(), event.event_kind.to_string()))
        .or_default() += 1;
}
