//! Lectures des news persistees retenues pour retraitements operateur.

use sqlx::{QueryBuilder, Sqlite};

use crate::{errors::AppError, features::news::domain::PersistedNewsEvent};

use super::super::super::{SqliteNewsStore, statements::NEWS_EVENT_SELECT};

impl SqliteNewsStore {
    /// Retourne les news retenues dont titre ou contenu peut matcher une entree glossaire active.
    pub async fn list_news_matching_active_glossary(
        &self,
        source: Option<&str>,
        limit: i64,
    ) -> Result<Vec<PersistedNewsEvent>, AppError> {
        let bounded_limit = limit.clamp(1, 2_000);
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_EVENT_SELECT);
        query_builder.push(" WHERE 1 = 1");

        if let Some(source) = source {
            query_builder.push(" AND source = ").push_bind(source);
        }

        query_builder.push(
            r#"
            AND EXISTS (
                SELECT 1
                FROM translation_glossary_entries AS glossary
                WHERE glossary.enabled = 1
                  AND glossary.scope = 'full_pipeline'
                  AND glossary.source_term <> ''
                  AND (
                    LOWER(COALESCE(news_events.title, '')) LIKE '%' || LOWER(REPLACE(REPLACE(glossary.source_term, '<number>', '%'), '<string>', '%')) || '%'
                    OR LOWER(COALESCE(news_events.body, '')) LIKE '%' || LOWER(REPLACE(REPLACE(glossary.source_term, '<number>', '%'), '<string>', '%')) || '%'
                  )
            )
            ORDER BY COALESCE(published_at, ingested_at) ASC, id ASC LIMIT "#,
        );
        query_builder.push_bind(bounded_limit);

        query_builder
            .build_query_as::<PersistedNewsEvent>()
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::from)
    }

    /// Retourne les news retenues dont titre ou contenu peut matcher une entree blacklist active.
    pub(crate) async fn list_news_matching_active_blacklist(
        &self,
        source: Option<&str>,
        limit: i64,
    ) -> Result<Vec<PersistedNewsEvent>, AppError> {
        let bounded_limit = limit.clamp(1, 2_000);
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_EVENT_SELECT);
        query_builder.push(" WHERE 1 = 1");

        if let Some(source) = source {
            query_builder.push(" AND source = ").push_bind(source);
        }

        query_builder.push(
            r#"
            AND EXISTS (
                SELECT 1
                FROM news_blacklist_entries AS entry
                WHERE entry.enabled = 1
                  AND entry.term <> ''
                  AND (
                    (
                      entry.target_scope IN ('title', 'title_content')
                      AND LOWER(COALESCE(news_events.title, '')) LIKE '%' || LOWER(entry.term) || '%'
                    )
                    OR (
                      entry.target_scope IN ('content', 'title_content')
                      AND LOWER(COALESCE(news_events.body, '')) LIKE '%' || LOWER(entry.term) || '%'
                    )
                  )
            )
            ORDER BY COALESCE(published_at, ingested_at) ASC, id ASC LIMIT "#,
        );
        query_builder.push_bind(bounded_limit);

        query_builder
            .build_query_as::<PersistedNewsEvent>()
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::from)
    }
}
