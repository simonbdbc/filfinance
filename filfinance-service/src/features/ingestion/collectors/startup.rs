//! Filtrage de bootstrap commun aux collecteurs legacy.

// `Date` permet de comparer les jours métier et `OffsetDateTime` parse/porte les timestamps RFC3339.
use time::{Date, OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use crate::features::news::domain::NewsEvent;

/// Résultat du filtrage de bootstrap au démarrage du service.
pub(in crate::features::ingestion) struct StartupBackfillBatch {
    /// Événements finalement émis dans le pipeline canonique.
    pub(in crate::features::ingestion) items: Vec<NewsEvent>,
    /// Nombre d'événements ignorés faute de date métier exploitable.
    pub(in crate::features::ingestion) skipped_without_business_date: usize,
    /// Nombre d'événements datés mais hors fenêtre J/J-1.
    pub(in crate::features::ingestion) skipped_outside_window: usize,
}

/// Filtre la récolte de bootstrap pour ne garder que les news datées sur J et J-1.
pub(in crate::features::ingestion) fn filter_startup_backfill_items(
    items: Vec<NewsEvent>,
    now: OffsetDateTime,
) -> StartupBackfillBatch {
    // Le service travaille en UTC pour garder une fenêtre stable quel que soit l'offset de la source.
    let today = now.to_offset(UtcOffset::UTC).date();
    // `previous_day()` peut retourner `None` au minimum théorique de `Date`; on dégrade alors sur `today`.
    let yesterday = today.previous_day().unwrap_or(today);
    let mut retained_items = Vec::new();
    let mut skipped_without_business_date = 0;
    let mut skipped_outside_window = 0;

    for item in items {
        let Some(business_date) = event_business_date(&item) else {
            // Sans date source fiable, on ne peut pas prouver que l'item appartient à J ou J-1.
            skipped_without_business_date += 1;
            continue;
        };

        if (yesterday..=today).contains(&business_date) {
            retained_items.push(item);
        } else {
            skipped_outside_window += 1;
        }
    }

    StartupBackfillBatch {
        items: retained_items,
        skipped_without_business_date,
        skipped_outside_window,
    }
}

/// Retourne la date métier d'un événement en priorisant `published_at`, puis `event_time`.
pub(in crate::features::ingestion) fn event_business_date(event: &NewsEvent) -> Option<Date> {
    event
        .published_at
        .as_deref()
        // `or(...)` ne lit `event_time` que si `published_at` est absent.
        .or(event.event_time.as_deref())
        .and_then(parse_rfc3339_business_date)
}

/// Parse un timestamp RFC3339 et le projette sur son jour UTC de référence.
pub(in crate::features::ingestion) fn parse_rfc3339_business_date(value: &str) -> Option<Date> {
    OffsetDateTime::parse(value, &Rfc3339)
        .ok()
        .map(|timestamp| timestamp.to_offset(UtcOffset::UTC).date())
}
