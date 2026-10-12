//! Application d'une réparation FinancialJuice sur une ligne news existante.

use crate::{errors::AppError, features::news::domain::NewsEvent, shared::time::now_rfc3339};

use super::super::super::SqliteNewsStore;

impl SqliteNewsStore {
    /// Applique les champs répares `FinancialJuice` a une ligne cible et synchronise ses traductions.
    pub(super) async fn apply_financial_juice_repair_to_event(
        &self,
        event: &NewsEvent,
        news_event_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE news_events
            SET
                title = ?1,
                url = ?2,
                published_at = ?3,
                body = ?4,
                news_level = ?5,
                raw_payload_ref = ?6,
                signature = ?7
            WHERE id = ?8
            "#,
        )
        .bind(&event.title)
        .bind(&event.url)
        .bind(&event.published_at)
        .bind(&event.body)
        .bind(&event.news_level)
        .bind(&event.raw_payload_ref)
        .bind(&event.signature)
        .bind(news_event_id)
        .execute(&self.pool)
        .await?;

        // Les traductions reutilisent ce `published_at` pour le tri historique et le replay.
        sqlx::query(
            r#"
            UPDATE news_translations
            SET
                source_title = ?1,
                source_body = ?2,
                published_at = ?3,
                updated_at = ?4
            WHERE news_event_id = ?5
            "#,
        )
        .bind(&event.title)
        .bind(&event.body)
        .bind(&event.published_at)
        .bind(now_rfc3339())
        .bind(news_event_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
