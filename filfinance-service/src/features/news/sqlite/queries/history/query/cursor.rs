//! Curseur SQL de l'historique des traductions news.

use sqlx::{QueryBuilder, Sqlite};

use crate::features::translation::domain::PersistedNewsTranslation;

/// Ajoute le curseur stable sur `(published_or_created_at, id)`.
pub(super) fn push_cursor_filter<'args>(
    query_builder: &mut QueryBuilder<'args, Sqlite>,
    cursor: &'args PersistedNewsTranslation,
) {
    let cursor_timestamp = cursor
        .published_at
        .as_deref()
        .unwrap_or(cursor.created_at.as_str());
    query_builder
        .push(" AND (COALESCE(news_translations.published_at, news_translations.created_at) < ")
        .push_bind(cursor_timestamp)
        .push(" OR (COALESCE(news_translations.published_at, news_translations.created_at) = ")
        .push_bind(cursor_timestamp)
        .push(" AND news_translations.id < ")
        .push_bind(cursor.id)
        .push("))");
}
