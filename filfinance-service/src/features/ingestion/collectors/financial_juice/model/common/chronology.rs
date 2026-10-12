//! Tri et déduplication des lots FinancialJuice.

// `HashSet` garde le dédoublonnage stable des lots historiques.
use std::collections::HashSet;

use crate::features::news::domain::NewsEvent;

/// Retourne une clé chronologique stable pour trier un lot historique.
fn best_event_timestamp(event: &NewsEvent) -> String {
    event
        .published_at
        .clone()
        .or_else(|| event.event_time.clone())
        .unwrap_or_else(|| event.ingested_at.clone())
}

/// Trie un lot chronologiquement puis supprime les doublons de signature éventuelle entre pages.
pub(in crate::features::ingestion::collectors::financial_juice) fn sort_and_dedup_events_chronologically(
    mut items: Vec<NewsEvent>,
) -> Vec<NewsEvent> {
    items.sort_by(|left, right| {
        best_event_timestamp(left)
            .cmp(&best_event_timestamp(right))
            .then_with(|| left.url.cmp(&right.url))
    });

    let mut seen_signatures = HashSet::new();
    items.retain(|item| seen_signatures.insert(item.signature.clone()));
    items
}
