//! Construction SQL de l'historique des traductions news.

mod cursor;
mod filters;

use sqlx::{QueryBuilder, Sqlite};

use crate::features::translation::domain::{PersistedNewsTranslation, TranslationHistoryParams};

use super::super::super::statements::NEWS_TRANSLATION_HISTORY_SELECT;
use cursor::push_cursor_filter;
use filters::{push_day_filter, push_scope_filter, push_source_filter};

/// Construit la requete paginee ou signale un curseur inexistant par `None`.
pub(super) fn build_translation_history_query<'args>(
    params: &'args TranslationHistoryParams,
    language: &'args str,
    cursor: Option<&'args PersistedNewsTranslation>,
) -> Option<QueryBuilder<'args, Sqlite>> {
    let mut query_builder = QueryBuilder::<Sqlite>::new(NEWS_TRANSLATION_HISTORY_SELECT);
    query_builder
        .push(" WHERE news_translations.language = ")
        .push_bind(language)
        .push(" AND news_translations.status = 'ready'");

    push_source_filter(&mut query_builder, params);
    push_day_filter(&mut query_builder, params);
    push_scope_filter(&mut query_builder, params.history_scope);

    if let Some(cursor) = cursor {
        push_cursor_filter(&mut query_builder, cursor);
    } else if params.before_id.is_some() {
        return None;
    }

    query_builder
        .push(" ORDER BY COALESCE(news_translations.published_at, news_translations.created_at) DESC, news_translations.id DESC LIMIT ")
        .push_bind(params.limit + 1);

    Some(query_builder)
}
