//! Historique SQLite des traductions persistées.

mod query;

use crate::{
    errors::AppError,
    features::translation::domain::{
        PersistedNewsTranslation, TranslationHistoryPage, TranslationHistoryParams,
    },
};

use super::super::SqliteNewsStore;
use query::build_translation_history_query;

impl SqliteNewsStore {
    /// Retourne une page de traductions persistentes avec séparation feed réel / test runs.
    pub async fn list_translation_history(
        &self,
        params: &TranslationHistoryParams,
        language: &str,
    ) -> Result<TranslationHistoryPage, AppError> {
        // Le curseur suit ici le tri de l'historique traduit, pas l'historique brut des news.
        let cursor = match params.before_id {
            Some(before_id) => {
                self.get_ready_translation_by_id_and_language(before_id, language)
                    .await?
            }
            None => None,
        };

        let Some(mut query_builder) =
            build_translation_history_query(params, language, cursor.as_ref())
        else {
            return Ok(TranslationHistoryPage::new(Vec::new(), false, None));
        };

        let mut items = query_builder
            .build_query_as::<PersistedNewsTranslation>()
            .fetch_all(&self.pool)
            .await?;
        let has_more = items.len() as i64 > params.limit;

        if has_more {
            items.pop();
        }

        let next_before_id = if has_more {
            items.last().map(|item| item.id)
        } else {
            None
        };

        Ok(TranslationHistoryPage::new(items, has_more, next_before_id))
    }
}
