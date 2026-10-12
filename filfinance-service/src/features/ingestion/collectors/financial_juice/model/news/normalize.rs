//! Normalisation des listes FinancialJuice live et backlog.

use crate::features::news::domain::NewsEvent;

use super::{
    item::FinancialJuiceApiNewsItem,
    logging::{log_observed_news_level, log_priority_like_unknown_fields},
    timestamp::FinancialJuiceBackfillTimestampCursor,
};

/// Normalise une liste d'items API FJ en `NewsEvent`.
pub(in crate::features::ingestion) fn normalize_financial_juice_news_items(
    source_name: &str,
    items: Vec<FinancialJuiceApiNewsItem>,
) -> Vec<NewsEvent> {
    items
        .into_iter()
        .filter_map(|item| {
            // `Level` est maintenant mappe: un log dedie remplace son ancienne observation via `extra`.
            log_observed_news_level(source_name, "sendUpdates", &item);
            // On audite les champs inconnus du live avant de consommer le DTO dans la conversion canonique.
            log_priority_like_unknown_fields(source_name, "sendUpdates", &item);
            // La conversion existante reste inchangée: seul le log d'observation est ajouté.
            item.into_news_event(source_name)
        })
        .collect()
}

/// Variante backlog qui recompose les jours UTC a partir des heures de `GetPreviousNews`.
pub(in crate::features::ingestion) fn normalize_financial_juice_history_items(
    source_name: &str,
    items: Vec<FinancialJuiceApiNewsItem>,
    timestamp_cursor: &mut FinancialJuiceBackfillTimestampCursor,
) -> Vec<NewsEvent> {
    items
        .into_iter()
        .filter_map(|item| {
            // Le backlog doit produire le meme inventaire `Level` que le live SignalR.
            log_observed_news_level(source_name, "GetPreviousNews", &item);
            // Le backlog `GetPreviousNews` partage le même DTO que le live; on y cherche aussi ces signaux.
            log_priority_like_unknown_fields(source_name, "GetPreviousNews", &item);
            // Le timestamp réparé garde le comportement historique du backfill et du catch-up.
            let repaired_published_at =
                timestamp_cursor.next_timestamp(item.date_published.as_deref());
            // La conversion canonique reste responsable de la validation titre/URL et de la signature.
            item.into_news_event_with_published_at(source_name, repaired_published_at)
        })
        .collect()
}
