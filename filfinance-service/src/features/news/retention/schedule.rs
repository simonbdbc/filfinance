//! Calculs d'horloge du worker de retention news.

// `Duration` exprime la cadence de purge sans entier ambigu.
use std::time::Duration;

// `OffsetDateTime` fournit l'instant local courant; `Time` fixe l'heure de purge.
use time::{OffsetDateTime, Time};
// Les avertissements rendent visible le fallback UTC sans bloquer le service.
use tracing::warn;

/// Heure locale de verification retention quotidienne.
const NEWS_RETENTION_DAILY_HOUR: u8 = 2;

/// Lit l'heure locale du process; en production elle doit rester configuree sur Europe/Paris.
pub(super) fn current_local_time() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|error| {
        warn!(%error, "failed to read local timezone for news retention, falling back to utc");
        OffsetDateTime::now_utc()
    })
}

/// Calcule l'attente avant la prochaine purge quotidienne a 02:00 dans l'offset local courant.
pub(in crate::features::news) fn duration_until_next_daily_run(now: OffsetDateTime) -> Duration {
    let target_time =
        Time::from_hms(NEWS_RETENTION_DAILY_HOUR, 0, 0).expect("02:00 is a valid wall-clock time");
    let target_date = if now.time() < target_time {
        now.date()
    } else {
        // `next_day` ne devrait echouer qu'a la borne maximale theorique du type `Date`.
        now.date().next_day().unwrap_or_else(|| now.date())
    };
    let next_run = target_date
        .with_time(target_time)
        .assume_offset(now.offset());
    let seconds = (next_run - now).whole_seconds().max(1) as u64;

    Duration::from_secs(seconds)
}
