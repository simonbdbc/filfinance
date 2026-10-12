//! Conversion des offsets navigateur FinancialJuice.

use time::UtcOffset;

/// Convertit l'offset minute de JS `getTimezoneOffset()` en `UtcOffset`.
pub(in crate::features::ingestion::collectors::financial_juice) fn time_offset_to_utc_offset(
    time_offset_minutes: i32,
) -> UtcOffset {
    let seconds = time_offset_minutes.saturating_mul(-60);
    UtcOffset::from_whole_seconds(seconds).unwrap_or(UtcOffset::UTC)
}
