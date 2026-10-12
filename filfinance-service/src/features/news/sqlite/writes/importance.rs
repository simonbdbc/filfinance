//! Backfill des importances dérivées du niveau FinancialJuice.

use super::super::SqliteNewsStore;
use crate::{errors::AppError, features::news::domain::financial_juice_importance_from_news_level};

impl SqliteNewsStore {
    /// Remplit les importances FinancialJuice manquantes sans écraser les valeurs existantes.
    pub async fn backfill_financial_juice_importance_from_news_level(
        &self,
    ) -> Result<u64, AppError> {
        let rows = sqlx::query_as::<_, (i64, Option<String>)>(
            r#"
            SELECT id, news_level
            FROM news_events
            WHERE source = 'financial_juice'
              AND (importance IS NULL OR trim(importance) = '');
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut updated = 0;
        for (id, news_level) in rows {
            let importance = financial_juice_importance_from_news_level(news_level.as_deref());
            let result = sqlx::query(
                r#"
                UPDATE news_events
                SET importance = ?1
                WHERE id = ?2
                  AND source = 'financial_juice'
                  AND (importance IS NULL OR trim(importance) = '');
                "#,
            )
            .bind(importance)
            .bind(id)
            .execute(&self.pool)
            .await?;
            updated += result.rows_affected();
        }

        Ok(updated)
    }
}
