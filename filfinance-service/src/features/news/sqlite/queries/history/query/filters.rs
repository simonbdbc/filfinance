//! Filtres SQL de l'historique des traductions news.

use sqlx::{QueryBuilder, Sqlite};

use crate::features::translation::domain::{TranslationHistoryParams, TranslationHistoryScope};

/// Ajoute le filtre source si la query HTTP l'a normalise.
pub(super) fn push_source_filter<'args>(
    query_builder: &mut QueryBuilder<'args, Sqlite>,
    params: &'args TranslationHistoryParams,
) {
    if let Some(source) = &params.source {
        query_builder
            .push(" AND news_translations.source = ")
            .push_bind(source);
    }
}

/// Ajoute le filtre jour sur le timestamp metier principal.
pub(super) fn push_day_filter<'args>(
    query_builder: &mut QueryBuilder<'args, Sqlite>,
    params: &'args TranslationHistoryParams,
) {
    if let Some(day) = &params.day {
        // Le préfixe `YYYY-MM-DD` reste suffisant car tous les timestamps sont RFC3339.
        query_builder
            .push(" AND substr(COALESCE(news_translations.published_at, news_translations.created_at), 1, 10) = ")
            .push_bind(day);
    }
}

/// Ajoute la séparation feed réel / run de test sans exposer SQL au handler.
pub(super) fn push_scope_filter(
    query_builder: &mut QueryBuilder<'_, Sqlite>,
    history_scope: TranslationHistoryScope,
) {
    match history_scope {
        TranslationHistoryScope::All => {}
        TranslationHistoryScope::RealFeed => {
            query_builder.push(
                " AND (news_events.raw_payload_ref IS NULL OR news_events.raw_payload_ref NOT LIKE 'test_run:%')",
            );
        }
        TranslationHistoryScope::TestRun => {
            query_builder.push(" AND news_events.raw_payload_ref LIKE 'test_run:%'");
        }
    }
}
