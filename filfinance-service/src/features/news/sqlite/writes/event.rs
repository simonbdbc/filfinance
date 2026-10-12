//! Insertion idempotente des evenements news SQLite.

mod duplicate;
mod lookup;

use super::super::SqliteNewsStore;
use crate::{
    errors::AppError,
    features::news::domain::{
        NewsEvent, PersistedNewsEvent, financial_juice_importance_from_news_level,
    },
};

impl SqliteNewsStore {
    /// Insere un evenement si sa signature n'existe pas deja, puis relit la ligne stockee.
    pub async fn insert_event(
        &self,
        event: &NewsEvent,
    ) -> Result<Option<PersistedNewsEvent>, AppError> {
        if let Some(existing_duplicate) = self
            .find_recent_financial_juice_text_duplicate(event)
            .await?
        {
            self.merge_financial_juice_text_duplicate(&existing_duplicate, event)
                .await?;
            return Ok(None);
        }

        // Les metadonnees macro sont eclatees dans le schema SQL pour simplifier les lectures MVP.
        // `as_ref()` passe de `Option<MacroMetadata>` a `Option<&MacroMetadata>` sans deplacer la valeur.
        let macro_metadata = event.macro_metadata.as_ref();
        let importance = if event.source == "financial_juice" {
            Some(financial_juice_importance_from_news_level(event.news_level.as_deref()).to_owned())
        } else {
            macro_metadata.and_then(|metadata| metadata.importance.clone())
        };
        // `INSERT OR IGNORE` s'appuie sur la contrainte unique `signature` pour l'idempotence durable.
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO news_events (
                source,
                event_kind,
                title,
                url,
                published_at,
                event_time,
                ingested_at,
                body,
                news_level,
                importance,
                country_code,
                currency,
                actual_value,
                forecast_value,
                previous_value,
                raw_payload_ref,
                signature
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17);
            "#,
        )
        .bind(&event.source)
        .bind(event.event_kind.as_str())
        .bind(&event.title)
        .bind(&event.url)
        .bind(&event.published_at)
        .bind(&event.event_time)
        .bind(&event.ingested_at)
        .bind(&event.body)
        .bind(&event.news_level)
        .bind(importance)
        .bind(macro_metadata.and_then(|metadata| metadata.country_code.clone()))
        .bind(macro_metadata.and_then(|metadata| metadata.currency.clone()))
        .bind(macro_metadata.and_then(|metadata| metadata.actual_value.clone()))
        .bind(macro_metadata.and_then(|metadata| metadata.forecast_value.clone()))
        .bind(macro_metadata.and_then(|metadata| metadata.previous_value.clone()))
        .bind(&event.raw_payload_ref)
        .bind(&event.signature)
        .execute(&self.pool)
        .await?;

        // Si aucune ligne n'a ete affectee, l'evenement etait deja connu via sa signature unique.
        if result.rows_affected() == 0 {
            // Un doublon peut arriver avec un `news_level` plus riche que la premiere occurrence.
            self.merge_news_level_for_duplicate(event).await?;
            return Ok(None);
        }

        // On relit la ligne apres insertion pour recuperer l'ID SQLite et la projection persistee complete.
        let stored = self.get_by_signature(&event.signature).await?;

        Ok(Some(stored))
    }
}
