//! Mesures temporelles non bloquantes pour le live audio.

use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Mesure un delai depuis un timestamp RFC3339, en laissant `None` si la source est invalide.
pub(in crate::features::audio::service::progressive) fn elapsed_since_rfc3339_ms(
    timestamp: &str,
) -> Option<u64> {
    let parsed = OffsetDateTime::parse(timestamp, &Rfc3339).ok()?;
    let elapsed_ms = (OffsetDateTime::now_utc() - parsed).whole_milliseconds();
    Some(elapsed_ms.max(0) as u64)
}
