//! Regles horaires appliquees a un token TTS.

use super::super::helpers::{french_minute_word, french_number_word, parse_time_token};

/// Rend les horaires `14:30` lisibles par une voix francaise.
pub(super) fn normalize_time_token(core: &str) -> Option<String> {
    let (hour, minute) = parse_time_token(core)?;
    if hour == 0 && minute == 0 {
        return Some("minuit".to_owned());
    }
    if hour == 12 && minute == 0 {
        return Some("midi".to_owned());
    }

    let hour_label = match hour {
        0 => "minuit".to_owned(),
        1 => "une heure".to_owned(),
        12 => "midi".to_owned(),
        _ => format!("{} heures", french_number_word(hour)?),
    };

    if minute == 0 {
        return Some(hour_label);
    }

    Some(format!("{hour_label} {}", french_minute_word(minute)?))
}
