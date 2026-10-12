//! Parsing des heures FinancialJuice.

use time::Time;

use super::super::text::non_empty;

/// Extrait uniquement l'heure UTC d'un `DatePublished` FinancialJuice.
pub(in crate::features::ingestion) fn parse_financial_juice_time_of_day(
    raw_timestamp: &str,
) -> Option<Time> {
    let timestamp = non_empty(raw_timestamp)?;
    let mut time_part = timestamp
        .rsplit_once('T')
        .map(|(_, value)| value)
        .unwrap_or(timestamp.as_str());

    // `GetPreviousNews` renvoie parfois un suffixe `Z` ou un offset; ici on ne garde que l'heure.
    time_part = time_part.trim_end_matches('Z');
    if let Some((without_offset, _)) = time_part.rsplit_once('+') {
        time_part = without_offset;
    } else if let Some((without_offset, offset_suffix)) = time_part.rsplit_once('-')
        && offset_suffix
            .chars()
            .all(|character| character.is_ascii_digit() || character == ':')
    {
        time_part = without_offset;
    }

    let mut segments = time_part.split(':');
    let hour = segments.next()?.parse::<u8>().ok()?;
    let minute = segments.next()?.parse::<u8>().ok()?;
    let second_segment = segments.next().unwrap_or("0");
    let (second_raw, fraction_raw) = second_segment
        .split_once('.')
        .unwrap_or((second_segment, ""));
    let second = second_raw.parse::<u8>().ok()?;

    let mut nanoseconds = 0_u32;
    if !fraction_raw.is_empty() {
        let padded_fraction = format!("{fraction_raw:0<9}");
        nanoseconds = padded_fraction.get(..9)?.parse::<u32>().ok()?;
    }

    Time::from_hms_nano(hour, minute, second, nanoseconds).ok()
}
