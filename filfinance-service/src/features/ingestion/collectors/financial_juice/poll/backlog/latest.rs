//! Catch-up periodique FinancialJuice depuis les pages recentes.

// OffsetDateTime sert a figer le moment de reference du curseur historique.
use time::OffsetDateTime;

use crate::{errors::AppError, features::news::domain::NewsEvent};

use super::super::super::{
    FINANCIAL_JUICE_PERIODIC_CATCHUP_PAGE_LIMIT, FinancialJuiceSiteCollector,
    model::{
        FinancialJuiceBackfillTimestampCursor, normalize_financial_juice_history_items,
        sort_and_dedup_events_chronologically,
    },
};

impl FinancialJuiceSiteCollector {
    /// Relit plusieurs pages recentes pour rattraper un eventuel trou entre deux cycles live.
    pub async fn poll_latest(&self) -> Result<Vec<NewsEvent>, AppError> {
        let page_limit = self
            .config
            .startup_backfill_page_limit
            .clamp(1, FINANCIAL_JUICE_PERIODIC_CATCHUP_PAGE_LIMIT);

        self.fetch_recent_pages(page_limit).await
    }

    /// Relit les `N` pages recentes de `GetPreviousNews` pour reconstituer un lot de rattrapage.
    async fn fetch_recent_pages(&self, page_limit: usize) -> Result<Vec<NewsEvent>, AppError> {
        let mut oldest_news_id = 0_i64;
        let mut previous_oldest_news_id = None;
        let mut retained_items = Vec::new();
        // Le catch-up periodique relit la meme API backlog que le bootstrap; on recompose donc
        // egalement ses jours UTC a partir des heures recues.
        let mut timestamp_cursor =
            FinancialJuiceBackfillTimestampCursor::new(OffsetDateTime::now_utc());

        for _ in 0..page_limit.max(1) {
            let Some(page) = self
                .fetch_backlog_page_or_stop(oldest_news_id, "periodic_catchup")
                .await?
            else {
                break;
            };

            if page.is_empty() {
                break;
            }

            retained_items.extend(normalize_financial_juice_history_items(
                &self.config.name,
                page.clone(),
                &mut timestamp_cursor,
            ));

            let Some(current_oldest_news_id) = page.iter().map(|item| item.news_id).min() else {
                break;
            };

            if Some(current_oldest_news_id) == previous_oldest_news_id {
                break;
            }

            previous_oldest_news_id = Some(current_oldest_news_id);
            oldest_news_id = current_oldest_news_id;
        }

        Ok(sort_and_dedup_events_chronologically(retained_items))
    }
}
