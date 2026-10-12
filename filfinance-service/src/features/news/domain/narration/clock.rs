//! Conversion des timestamps source en libellé horaire produit.

// Les types time calculent l'heure Europe/Paris sans dependre du fuseau du conteneur.
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use crate::shared::time::europe_paris_offset_for_utc;

/// Convertit un horodatage RFC3339 en `HH:MM` dans le fuseau produit Europe/Paris.
pub(super) fn format_local_clock_label(timestamp: &str) -> Option<String> {
    // `ok()?` convertit le `Result` du parse en `Option` et sort tot si la date est invalide.
    let parsed = OffsetDateTime::parse(timestamp, &Rfc3339).ok()?;
    // Le serveur peut tourner dans un conteneur UTC; le produit, lui, parle en heure Europe/Paris.
    let local_offset = europe_paris_offset_for_utc(parsed).unwrap_or_else(|| parsed.offset());

    Some(format_clock_label_for_offset(parsed, local_offset))
}

/// Formate un `OffsetDateTime` deja parse dans un offset cible.
pub fn format_clock_label_for_offset(timestamp: OffsetDateTime, offset: UtcOffset) -> String {
    // `to_offset` ne change pas l'instant absolu, seulement sa representation horaire locale.
    let local_timestamp = timestamp.to_offset(offset);
    // `format!` reste le moyen le plus lisible ici pour imposer un rendu fixe `HH:MM`.
    format!(
        "{:02}:{:02}",
        local_timestamp.hour(),
        local_timestamp.minute()
    )
}
