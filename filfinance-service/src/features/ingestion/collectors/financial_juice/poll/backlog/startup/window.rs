//! Fenetre temporelle J/J-1 du backfill de demarrage FinancialJuice.

use time::{Date, OffsetDateTime, UtcOffset};

use crate::features::{
    ingestion::collectors::financial_juice::model::{
        event_business_date_for_offset, time_offset_to_utc_offset,
    },
    news::domain::NewsEvent,
};

/// Fenetre metier retenue pendant le backfill initial.
pub(super) struct StartupBackfillWindow {
    /// Offset reconstruit depuis la configuration legacy `TimeOffset`.
    offset: UtcOffset,
    /// Jour metier courant pour le flux FinancialJuice.
    today: Date,
    /// Jour metier precedent encore conserve au demarrage.
    yesterday: Date,
}

impl StartupBackfillWindow {
    /// Construit la fenetre J/J-1 a partir de l'offset navigateur FinancialJuice.
    pub(super) fn from_time_offset_minutes(time_offset_minutes: i32) -> Self {
        let offset = time_offset_to_utc_offset(time_offset_minutes);
        // `OffsetDateTime::now_utc()` donne une heure universelle puis `to_offset(...)` la projette
        // dans le fuseau "metier" attendu pour savoir ce que `J` et `J-1` signifient vraiment.
        let today = OffsetDateTime::now_utc().to_offset(offset).date();
        // `previous_day()` retourne un `Option<Date>` car le jour precedent pourrait theoriquement
        // sortir du domaine representable; `unwrap_or(today)` evite toute panique inutile.
        let yesterday = today.previous_day().unwrap_or(today);

        Self {
            offset,
            today,
            yesterday,
        }
    }

    /// Classe un evenement normalise selon la fenetre conservee au bootstrap.
    pub(super) fn classify(&self, event: &NewsEvent) -> StartupWindowMatch {
        // `let Some(...) else` garde la lecture lineaire: si l'evenement n'a pas de date
        // metier exploitable, on l'ignore immediatement sans imbriquer un `match`.
        let Some(business_date) = event_business_date_for_offset(event, self.offset) else {
            return StartupWindowMatch::Ignored;
        };

        // Le range inclusif `yesterday..=today` rend explicite que J et J-1 sont tous deux gardes.
        if (self.yesterday..=self.today).contains(&business_date) {
            StartupWindowMatch::Retained
        } else if business_date < self.yesterday {
            StartupWindowMatch::OlderThanWindow
        } else {
            StartupWindowMatch::Ignored
        }
    }
}

/// Resultat de classification d'un item normalise par rapport a J/J-1.
pub(super) enum StartupWindowMatch {
    /// L'evenement est dans J ou J-1 et doit etre persiste.
    Retained,
    /// L'evenement est plus vieux que J-1 et peut contribuer a l'arret du backfill.
    OlderThanWindow,
    /// L'evenement est inexploitable ou hors fenetre sans prouver que la pagination est trop vieille.
    Ignored,
}
