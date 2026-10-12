//! Dates métier des événements FinancialJuice.

use time::{Date, OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use crate::features::news::domain::NewsEvent;

/// Retourne la date métier d'un événement selon l'offset de référence voulu.
pub(in crate::features::ingestion::collectors::financial_juice) fn event_business_date_for_offset(
    event: &NewsEvent,
    offset: UtcOffset,
) -> Option<Date> {
    event
        .published_at
        .as_deref()
        .or(event.event_time.as_deref())
        .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
        .map(|timestamp| timestamp.to_offset(offset).date())
}
