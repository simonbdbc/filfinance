//! Normalisation RFC3339 des timestamps FinancialJuice.

use time::{
    OffsetDateTime,
    format_description::well_known::{Rfc2822, Rfc3339},
};

use super::super::text::non_empty;

/// Convertit une date source en RFC3339 quand c'est possible.
pub(in crate::features::ingestion::collectors::financial_juice) fn normalize_timestamp(
    raw_timestamp: &str,
) -> Option<String> {
    let timestamp = non_empty(raw_timestamp)?;

    if let Ok(parsed) = OffsetDateTime::parse(&timestamp, &Rfc2822)
        .or_else(|_| OffsetDateTime::parse(&timestamp, &Rfc3339))
    {
        return parsed.format(&Rfc3339).ok();
    }

    if timestamp.contains('T')
        && !timestamp.ends_with('Z')
        && !timestamp
            .rsplit_once('T')
            .map(|(_, time_part)| time_part.contains('+') || time_part.contains('-'))
            .unwrap_or(false)
    {
        return Some(format!("{timestamp}Z"));
    }

    Some(timestamp)
}
