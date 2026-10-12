//! Parsing des horaires dans les narrations TTS.

/// Parse un horaire strict `HH:MM` ou `H:MM` sans interpreter d'autres ratios.
pub(in crate::features::audio::tts::normalization) fn parse_time_token(
    core: &str,
) -> Option<(u32, u32)> {
    let (hour, minute) = core.split_once(':')?;
    if hour.is_empty()
        || hour.len() > 2
        || minute.len() != 2
        || !hour.chars().all(|ch| ch.is_ascii_digit())
        || !minute.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let hour = hour.parse::<u32>().ok()?;
    let minute = minute.parse::<u32>().ok()?;
    if hour <= 23 && minute <= 59 {
        Some((hour, minute))
    } else {
        None
    }
}
