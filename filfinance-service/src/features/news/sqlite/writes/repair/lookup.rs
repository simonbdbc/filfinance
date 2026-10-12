//! Lectures ciblées utilisées par les réparations FinancialJuice.

use sqlx::{QueryBuilder, Sqlite};

use crate::{errors::AppError, features::news::domain::PersistedNewsEvent};

use super::super::super::{SqliteNewsStore, statements::NEWS_EVENT_SELECT};

impl SqliteNewsStore {
    /// Variante optionnelle pour les reparations qui doivent tester une collision de signature.
    pub(super) async fn get_optional_by_signature(
        &self,
        signature: &str,
    ) -> Result<Option<PersistedNewsEvent>, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_EVENT_SELECT);
        query_builder
            .push(" WHERE signature = ")
            .push_bind(signature)
            .push(" LIMIT 1");

        let event = query_builder
            .build_query_as::<PersistedNewsEvent>()
            .fetch_optional(&self.pool)
            .await?;

        Ok(event)
    }

    /// Relit un evenement par `(source, raw_payload_ref)` quand la source expose un identifiant stable.
    pub(super) async fn get_by_source_and_raw_payload_ref(
        &self,
        source: &str,
        raw_payload_ref: &str,
    ) -> Result<Option<PersistedNewsEvent>, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_EVENT_SELECT);
        query_builder
            .push(" WHERE source = ")
            .push_bind(source)
            .push(" AND raw_payload_ref = ")
            .push_bind(raw_payload_ref)
            .push(" ORDER BY id DESC LIMIT 1");

        let event = query_builder
            .build_query_as::<PersistedNewsEvent>()
            .fetch_optional(&self.pool)
            .await?;

        Ok(event)
    }
}
