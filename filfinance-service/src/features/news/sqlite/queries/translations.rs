//! Lectures SQLite ciblées des traductions news.

use sqlx::{QueryBuilder, Sqlite};

use crate::{errors::AppError, features::translation::domain::PersistedNewsTranslation};

use super::super::{SqliteNewsStore, statements::NEWS_TRANSLATION_SELECT};

impl SqliteNewsStore {
    /// Retourne une variante texte unique pour un événement et une langue cibles.
    pub async fn get_translation_by_news_event_id_and_language(
        &self,
        news_event_id: i64,
        language: &str,
    ) -> Result<Option<PersistedNewsTranslation>, AppError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_TRANSLATION_SELECT);
        // Le couple `(news_event_id, language)` est unique par schéma SQL.
        query_builder
            .push(" WHERE news_event_id = ")
            .push_bind(news_event_id)
            .push(" AND language = ")
            .push_bind(language)
            .push(" LIMIT 1");

        let item = query_builder
            .build_query_as::<PersistedNewsTranslation>()
            .fetch_optional(&self.pool)
            .await?;

        Ok(item)
    }

    /// Retourne une traduction "ready" unique à partir de son identifiant primaire.
    pub async fn get_ready_translation_by_id_and_language(
        &self,
        translation_id: i64,
        language: &str,
    ) -> Result<Option<PersistedNewsTranslation>, AppError> {
        // Cette lecture ciblée sert à rejouer un item audio précis sans rescanner tout l'historique.
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_TRANSLATION_SELECT);
        query_builder
            .push(" WHERE id = ")
            .push_bind(translation_id)
            .push(" AND language = ")
            .push_bind(language)
            .push(" AND status = 'ready'")
            .push(" LIMIT 1");

        let item = query_builder
            .build_query_as::<PersistedNewsTranslation>()
            .fetch_optional(&self.pool)
            .await?;

        Ok(item)
    }

    /// Retourne les variantes texte les plus récentes selon `published_at`, dans l'ordre chronologique.
    pub async fn list_recent_translations(
        &self,
        language: &str,
        limit: i64,
    ) -> Result<Vec<PersistedNewsTranslation>, AppError> {
        // On borne défensivement la taille du replay récent pour les ouvertures de sockets.
        let bounded_limit = limit.clamp(1, 50);
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_TRANSLATION_SELECT);
        query_builder
            .push(" WHERE language = ")
            .push_bind(language)
            .push(" AND status = 'ready'")
            .push(" ORDER BY COALESCE(published_at, created_at) DESC, id DESC LIMIT ")
            .push_bind(bounded_limit);

        let mut items = query_builder
            .build_query_as::<PersistedNewsTranslation>()
            .fetch_all(&self.pool)
            .await?;

        // Le SQL lit d'abord les plus récentes; on inverse ensuite pour rejouer du plus ancien au plus récent.
        items.reverse();
        Ok(items)
    }
}
