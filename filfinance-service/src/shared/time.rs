//! Helpers temporels transverses conserves hors des domaines metier.

// `OffsetDateTime` lit l'horloge UTC; `Rfc3339` fixe le format texte commun du service.
use time::{
    Date, Month, OffsetDateTime, Time, UtcOffset, Weekday, format_description::well_known::Rfc3339,
};

/// Normalise l'horodatage courant dans le format unique du service.
pub(crate) fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        // `format(&Rfc3339)` peut echouer theoriquement; le fallback evite toute panique.
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

/// Retourne l'offset Europe/Paris applicable a un instant UTC.
pub(crate) fn europe_paris_offset_for_utc(timestamp: OffsetDateTime) -> Option<UtcOffset> {
    // On compare toujours les transitions en UTC, comme la regle officielle de changement d'heure UE.
    let utc_timestamp = timestamp.to_offset(UtcOffset::UTC);
    // L'annee UTC suffit car les transitions Paris sont exprimees en UTC.
    let year = utc_timestamp.year();
    // Le passage a l'heure d'ete a lieu le dernier dimanche de mars a 01:00 UTC.
    let daylight_start = europe_paris_dst_transition_utc(year, Month::March, 31)?;
    // Le retour a l'heure d'hiver a lieu le dernier dimanche d'octobre a 01:00 UTC.
    let daylight_end = europe_paris_dst_transition_utc(year, Month::October, 31)?;
    // Entre ces bornes, Paris est en UTC+2; sinon en UTC+1.
    let offset_hours: i8 = if utc_timestamp >= daylight_start && utc_timestamp < daylight_end {
        2
    } else {
        1
    };

    UtcOffset::from_hms(offset_hours, 0, 0).ok()
}

/// Retourne le jour produit Europe/Paris pour un instant UTC.
pub(crate) fn europe_paris_day_for_utc(timestamp: OffsetDateTime) -> Option<Date> {
    let offset = europe_paris_offset_for_utc(timestamp)?;
    Some(timestamp.to_offset(offset).date())
}

/// Calcule une transition Europe/Paris exprimee a 01:00 UTC sur le dernier dimanche du mois.
fn europe_paris_dst_transition_utc(
    year: i32,
    month: Month,
    last_day_of_month: u8,
) -> Option<OffsetDateTime> {
    // Le dernier dimanche change chaque annee; on le calcule plutot que de figer une table.
    let transition_date = last_sunday_of_month(year, month, last_day_of_month)?;
    // `Time::from_hms` garde le contrat explicite: la transition officielle est a 01:00 UTC.
    let transition_time = Time::from_hms(1, 0, 0).ok()?;
    // `assume_utc` transforme le couple date/heure en instant comparable aux timestamps source.
    Some(transition_date.with_time(transition_time).assume_utc())
}

/// Trouve le dernier dimanche d'un mois donne.
fn last_sunday_of_month(year: i32, month: Month, last_day_of_month: u8) -> Option<Date> {
    // On part du dernier jour connu du mois, puis on recule jusqu'au dimanche.
    let mut date = Date::from_calendar_date(year, month, last_day_of_month).ok()?;
    while date.weekday() != Weekday::Sunday {
        date = date.previous_day()?;
    }

    Some(date)
}
