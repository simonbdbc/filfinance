//! Horloge locale et validation de jour HLS.

use crate::shared::time::europe_paris_day_for_utc;

// `SystemTime` sert aux expirations millisecondes des clips.
use std::time::{SystemTime, UNIX_EPOCH};

use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Retourne le jour produit Europe/Paris, independant de la timezone du conteneur.
pub(in crate::features::audio) fn current_local_day() -> String {
    local_day_for_utc(OffsetDateTime::now_utc())
}

/// Retourne le jour HLS produit pour un instant UTC donne.
pub(in crate::features::audio) fn local_day_for_utc(timestamp: OffsetDateTime) -> String {
    // Le replay est une fonctionnalite produit: le jour attendu est Europe/Paris, pas `TZ` Docker.
    europe_paris_day_for_utc(timestamp)
        .unwrap_or_else(|| timestamp.to_offset(time::UtcOffset::UTC).date())
        .to_string()
}

/// Retourne le jour HLS produit pour un timestamp metier RFC3339.
pub(in crate::features::audio) fn local_day_for_rfc3339(value: &str) -> Option<String> {
    let timestamp = OffsetDateTime::parse(value, &Rfc3339).ok()?;
    Some(local_day_for_utc(timestamp))
}

/// Convertit `SystemTime` en millisecondes Unix positives.
fn system_time_to_unix_ms(value: SystemTime) -> Option<i64> {
    // Les dates avant epoch ne sont pas attendues pour des segments HLS et sont ignorees.
    let duration = value.duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(duration.as_millis()).ok()
}

/// Retourne l'horloge courante en millisecondes Unix pour les mutations de clip.
pub(in crate::features::audio) fn now_unix_ms() -> i64 {
    // `SystemTime` garde la precision milliseconde utile pour calculer une expiration courte.
    system_time_to_unix_ms(SystemTime::now()).unwrap_or_else(|| {
        time::OffsetDateTime::now_utc()
            .unix_timestamp()
            .saturating_mul(1000)
    })
}

/// Verifie le format strict `YYYY-MM-DD` des dossiers jour HLS.
pub(in crate::features::audio::hls::origin) fn looks_like_local_day(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[0..4].iter().all(|byte| byte.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|byte| byte.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|byte| byte.is_ascii_digit())
}
