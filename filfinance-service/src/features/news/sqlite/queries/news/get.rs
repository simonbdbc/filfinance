//! Lecture SQLite d'un evenement news par identifiant.

use sqlx::{QueryBuilder, Sqlite};

use crate::{errors::AppError, features::news::domain::PersistedNewsEvent};

use super::super::super::{SqliteNewsStore, statements::NEWS_EVENT_SELECT};

impl SqliteNewsStore {
    /// Retourne un evenement specifique par identifiant pour les besoins de debug.
    pub async fn get_news_by_id(&self, id: i64) -> Result<Option<PersistedNewsEvent>, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_EVENT_SELECT);
        // Le `LIMIT 1` exprime clairement l'intention et protege d'un eventuel SQL trop large.
        query_builder
            .push(" WHERE id = ")
            .push_bind(id)
            .push(" LIMIT 1");

        let item = query_builder
            .build_query_as::<PersistedNewsEvent>()
            .fetch_optional(&self.pool)
            .await?;

        Ok(item)
    }
}
