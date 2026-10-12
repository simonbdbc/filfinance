//! Relecture ciblee des evenements news apres insertion.

use sqlx::{QueryBuilder, Sqlite};

use crate::{
    errors::AppError,
    features::news::{
        domain::PersistedNewsEvent,
        sqlite::{SqliteNewsStore, statements::NEWS_EVENT_SELECT},
    },
};

impl SqliteNewsStore {
    /// Relit une ligne par signature pour renvoyer exactement l'objet persiste.
    pub(super) async fn get_by_signature(
        &self,
        signature: &str,
    ) -> Result<PersistedNewsEvent, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_EVENT_SELECT);
        query_builder
            .push(" WHERE signature = ")
            .push_bind(signature)
            .push(" LIMIT 1");
        let event = query_builder
            .build_query_as::<PersistedNewsEvent>()
            // `fetch_one` est adapte ici car une signature inseree juste avant doit exister exactement une fois.
            .fetch_one(&self.pool)
            .await?;

        Ok(event)
    }
}
