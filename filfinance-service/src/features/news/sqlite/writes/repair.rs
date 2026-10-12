//! Reparations idempotentes des lignes FinancialJuice deja persistees.

mod apply;
mod lookup;
mod merge;

use crate::{errors::AppError, features::news::domain::NewsEvent};

use super::super::SqliteNewsStore;

impl SqliteNewsStore {
    /// Répare un lot `FinancialJuice` déjà persistant en recalant ses timestamps via `raw_payload_ref`.
    pub async fn repair_financial_juice_events(
        &self,
        events: &[NewsEvent],
    ) -> Result<(), AppError> {
        for event in events {
            // Le correctif vise uniquement le collecteur `FinancialJuice` primaire et suppose
            // une reference brute stable issue de `NewsID` ou `IID`.
            if event.source != "financial_juice" {
                continue;
            }

            let Some(raw_payload_ref) = event.raw_payload_ref.as_deref() else {
                continue;
            };

            let Some(stored_event) = self
                .get_by_source_and_raw_payload_ref(&event.source, raw_payload_ref)
                .await?
            else {
                continue;
            };

            // Si rien ne differe, on evite une ecriture SQLite inutile.
            if stored_event.title == event.title
                && stored_event.url == event.url
                && stored_event.published_at == event.published_at
                && stored_event.body == event.body
                && stored_event.news_level == event.news_level
                && stored_event.signature == event.signature
            {
                continue;
            }

            if let Some(signature_owner) = self.get_optional_by_signature(&event.signature).await?
                && signature_owner.id != stored_event.id
            {
                self.merge_financial_juice_repair_duplicate(
                    event,
                    stored_event.id,
                    signature_owner.id,
                )
                .await?;
                continue;
            }

            self.apply_financial_juice_repair_to_event(event, stored_event.id)
                .await?;
        }

        Ok(())
    }
}
