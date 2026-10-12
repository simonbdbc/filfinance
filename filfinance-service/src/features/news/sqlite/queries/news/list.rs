//! Pagination SQLite des evenements news.

use sqlx::{QueryBuilder, Sqlite};

use crate::{
    errors::AppError,
    features::news::domain::{NewsListParams, NewsListResponse, PersistedNewsEvent},
};

use super::super::super::{SqliteNewsStore, statements::NEWS_EVENT_SELECT};

impl SqliteNewsStore {
    /// Retourne une page d'historique en appliquant les filtres MVP et un curseur simple.
    pub async fn list_news(&self, params: &NewsListParams) -> Result<NewsListResponse, AppError> {
        // `QueryBuilder::<Sqlite>` encode explicitement le backend cible du SQL dynamique.
        let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_EVENT_SELECT);
        // Quand `before_id` existe, on relit d'abord son evenement pour transformer le curseur `id`
        // en vrai curseur temporel fonde sur `published_at`/`ingested_at`.
        let cursor = match params.before_id {
            Some(before_id) => self.get_news_by_id(before_id).await?,
            None => None,
        };

        // Les filtres sont ajoutes dynamiquement pour eviter plusieurs variantes SQL quasi identiques.
        query_builder.push(" WHERE 1 = 1");

        if let Some(source) = &params.source {
            // `push_bind(source)` fait un bind parametre SQLx au lieu de concatener la valeur en dur.
            query_builder.push(" AND source = ").push_bind(source);
        }

        if let Some(event_kind) = &params.event_kind {
            query_builder
                .push(" AND event_kind = ")
                .push_bind(event_kind);
        }

        if let Some(cursor) = cursor.as_ref() {
            // Le curseur reel est la date visible cote produit, avec fallback sur l'ingestion.
            let cursor_published_at = cursor
                .published_at
                .as_deref()
                .unwrap_or(cursor.ingested_at.as_str());
            query_builder
                .push(" AND (COALESCE(published_at, ingested_at) < ")
                .push_bind(cursor_published_at)
                .push(" OR (COALESCE(published_at, ingested_at) = ")
                .push_bind(cursor_published_at)
                .push(" AND id < ")
                .push_bind(cursor.id)
                .push("))");
        } else if params.before_id.is_some() {
            // Si le curseur demande n'existe pas, on retourne simplement une page vide stable.
            return Ok(NewsListResponse::new(Vec::new(), false, None));
        }

        // Le tri suit l'horodatage de publication visible; `id DESC` garde un ordre stable a egalite.
        query_builder
            .push(" ORDER BY COALESCE(published_at, ingested_at) DESC, id DESC LIMIT ")
            .push_bind(params.limit + 1);

        let mut items = query_builder
            .build_query_as::<PersistedNewsEvent>()
            .fetch_all(&self.pool)
            .await?;
        // On demande `limit + 1` lignes pour savoir si une page suivante existe.
        let has_more = items.len() as i64 > params.limit;

        if has_more {
            // On retire la ligne surnumeraire utilisee uniquement pour detecter `has_more`.
            items.pop();
        }

        // `next_before_id` reutilise l'ID du dernier item reel de la page courante.
        let next_before_id = if has_more {
            items.last().map(|item| item.id)
        } else {
            None
        };

        Ok(NewsListResponse::new(items, has_more, next_before_id))
    }
}
