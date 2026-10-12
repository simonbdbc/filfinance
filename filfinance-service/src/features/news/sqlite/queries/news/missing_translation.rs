//! Lectures des news sans traduction cible prete.

use sqlx::{QueryBuilder, Sqlite};

use crate::{errors::AppError, features::news::domain::PersistedNewsEvent};

use super::super::super::{SqliteNewsStore, statements::NEWS_EVENT_QUALIFIED_SELECT};

impl SqliteNewsStore {
    /// Retourne les news persistees d'une fenetre de jours qui n'ont pas encore de traduction cible.
    pub async fn list_news_missing_translation_for_days(
        &self,
        source: Option<&str>,
        language: &str,
        days: &[String],
        limit: i64,
    ) -> Result<Vec<PersistedNewsEvent>, AppError> {
        if days.is_empty() {
            return Ok(Vec::new());
        }

        let bounded_limit = limit.clamp(1, 2_000);
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_EVENT_QUALIFIED_SELECT);
        query_builder
            .push(
                " LEFT JOIN news_translations ON news_translations.news_event_id = news_events.id",
            )
            .push(" AND news_translations.language = ")
            .push_bind(language)
            .push(" AND news_translations.status = 'ready'")
            .push(" WHERE news_translations.id IS NULL");

        if let Some(source) = source {
            query_builder
                .push(" AND news_events.source = ")
                .push_bind(source);
        }

        query_builder.push(
            " AND substr(COALESCE(news_events.published_at, news_events.ingested_at), 1, 10) IN (",
        );

        {
            let mut separated = query_builder.separated(", ");
            for day in days {
                separated.push_bind(day);
            }
        }

        query_builder
            .push(")")
            .push(" ORDER BY COALESCE(news_events.published_at, news_events.ingested_at) ASC, news_events.id ASC LIMIT ")
            .push_bind(bounded_limit);

        query_builder
            .build_query_as::<PersistedNewsEvent>()
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::from)
    }
}
