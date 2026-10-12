//! Fusion d'un doublon FinancialJuice détecté pendant une réparation.

use crate::{errors::AppError, features::news::domain::NewsEvent, shared::time::now_rfc3339};

use super::super::super::SqliteNewsStore;

impl SqliteNewsStore {
    /// Fusionne une ancienne ligne `FinancialJuice` vers la ligne qui porte déjà la signature cible.
    pub(super) async fn merge_financial_juice_repair_duplicate(
        &self,
        event: &NewsEvent,
        duplicate_news_event_id: i64,
        target_news_event_id: i64,
    ) -> Result<(), AppError> {
        self.apply_financial_juice_repair_to_event(event, target_news_event_id)
            .await?;

        sqlx::query(
            r#"
            UPDATE news_translations
            SET
                news_event_id = ?1,
                source_title = ?3,
                source_body = ?4,
                published_at = ?5,
                updated_at = ?6
            WHERE news_event_id = ?2
              AND NOT EXISTS (
                SELECT 1
                FROM news_translations AS existing
                WHERE existing.news_event_id = ?1
                  AND existing.language = news_translations.language
              )
            "#,
        )
        .bind(target_news_event_id)
        .bind(duplicate_news_event_id)
        .bind(&event.title)
        .bind(&event.body)
        .bind(&event.published_at)
        .bind(now_rfc3339())
        .execute(&self.pool)
        .await?;

        sqlx::query("DELETE FROM news_translations WHERE news_event_id = ?1")
            .bind(duplicate_news_event_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM news_events WHERE id = ?1")
            .bind(duplicate_news_event_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
