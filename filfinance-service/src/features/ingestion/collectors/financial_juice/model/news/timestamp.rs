//! Curseur de reconstruction des timestamps UTC du backlog FinancialJuice.

// Date, Time et OffsetDateTime gardent la reconstruction du backlog UTC.
use time::{Date, OffsetDateTime, Time, format_description::well_known::Rfc3339};

use super::super::common::parse_financial_juice_time_of_day;

/// Curseur qui recompose une chronologie UTC plausible a partir des heures renvoyees par `GetPreviousNews`.
#[derive(Debug, Clone)]
pub(in crate::features::ingestion) struct FinancialJuiceBackfillTimestampCursor {
    /// Jour UTC courant auquel on rattache les items du lot tant qu'on ne repasse pas minuit.
    current_day_utc: Date,
    /// Heure du "maintenant" UTC au moment de la requete, utile pour le tout premier item.
    reference_time_utc: Time,
    /// Heure du dernier item traite (plus recent) pour detecter un franchissement de minuit.
    previous_item_time_utc: Option<Time>,
}

impl FinancialJuiceBackfillTimestampCursor {
    /// Initialise le curseur au "maintenant" UTC de l'appelant.
    pub(in crate::features::ingestion) fn new(reference_utc: OffsetDateTime) -> Self {
        Self {
            current_day_utc: reference_utc.date(),
            reference_time_utc: reference_utc.time(),
            previous_item_time_utc: None,
        }
    }

    /// Recompose un horodatage UTC stable a partir de l'heure source renvoyee par `GetPreviousNews`.
    pub(in crate::features::ingestion::collectors::financial_juice::model::news) fn next_timestamp(
        &mut self,
        raw_timestamp: Option<&str>,
    ) -> Option<String> {
        let item_time_utc = parse_financial_juice_time_of_day(raw_timestamp?)?;

        // Le premier item de la page est le plus recent. S'il "est dans le futur" par rapport
        // au moment UTC courant, cela signifie en pratique qu'il appartient encore a la veille.
        if self.previous_item_time_utc.is_none() && item_time_utc > self.reference_time_utc {
            self.current_day_utc = self
                .current_day_utc
                .previous_day()
                .unwrap_or(self.current_day_utc);
        }

        // Ensuite, chaque fois qu'un item plus ancien porte une heure plus grande que l'item
        // precedent, on a franchi minuit en remontant le backlog et il faut donc reculer d'un jour.
        if let Some(previous_item_time_utc) = self.previous_item_time_utc
            && item_time_utc > previous_item_time_utc
        {
            self.current_day_utc = self
                .current_day_utc
                .previous_day()
                .unwrap_or(self.current_day_utc);
        }

        self.previous_item_time_utc = Some(item_time_utc);

        self.current_day_utc
            .with_time(item_time_utc)
            .assume_utc()
            .format(&Rfc3339)
            .ok()
    }
}
