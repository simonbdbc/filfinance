//! Merge minimal des doublons d'evenements news.

use crate::{
    errors::AppError,
    features::news::{
        domain::{
            NewsEvent, PersistedNewsEvent, financial_juice_importance_from_news_level,
            financial_juice_title_key, financial_juice_titles_are_near_duplicates,
            is_financial_juice_family,
        },
        sqlite::SqliteNewsStore,
    },
    shared::time::now_rfc3339,
};

use super::super::super::statements::NEWS_EVENT_SELECT;

const FINANCIAL_JUICE_RECENT_TEXT_DUPLICATE_LOOKBACK: i64 = 5;

impl SqliteNewsStore {
    /// Retrouve un doublon texte parmi les cinq derniers messages FinancialJuice.
    pub(super) async fn find_recent_financial_juice_text_duplicate(
        &self,
        event: &NewsEvent,
    ) -> Result<Option<PersistedNewsEvent>, AppError> {
        if !is_financial_juice_family(&event.source) {
            return Ok(None);
        }

        let incoming_key = financial_juice_title_key(&event.title);
        if incoming_key.is_empty() {
            return Ok(None);
        }

        let query = format!(
            r#"
            {NEWS_EVENT_SELECT}
            WHERE source IN ('financial_juice', 'financial_juice_browser')
              AND event_kind = ?1
            ORDER BY id DESC
            LIMIT ?2;
            "#
        );
        let candidates = sqlx::query_as::<_, PersistedNewsEvent>(&query)
            .bind(event.event_kind.as_str())
            .bind(FINANCIAL_JUICE_RECENT_TEXT_DUPLICATE_LOOKBACK)
            .fetch_all(&self.pool)
            .await?;

        for candidate in candidates {
            let candidate_key = financial_juice_title_key(&candidate.title);
            if candidate_key != incoming_key
                && !financial_juice_titles_are_near_duplicates(&candidate.title, &event.title)
            {
                continue;
            }

            return Ok(Some(candidate));
        }

        Ok(None)
    }

    /// Merge minimal d'un doublon texte qui n'a pas exactement la meme signature durable.
    pub(super) async fn merge_financial_juice_text_duplicate(
        &self,
        existing: &PersistedNewsEvent,
        event: &NewsEvent,
    ) -> Result<(), AppError> {
        if event.source == "financial_juice" && existing.source == "financial_juice_browser" {
            self.promote_financial_juice_duplicate_by_id(existing.id, event)
                .await?;
            return Ok(());
        }

        if event.source == "financial_juice_browser" {
            return Ok(());
        }

        let Some(news_level) = event.news_level.as_deref() else {
            if event.source == "financial_juice" {
                self.fill_missing_financial_juice_importance_by_id(existing.id)
                    .await?;
            }
            return Ok(());
        };
        let importance = financial_juice_importance_from_news_level(Some(news_level));

        sqlx::query(
            r#"
            UPDATE news_events
            SET news_level = ?1,
                importance = ?2
            WHERE id = ?3
              AND (news_level IS NULL OR news_level != ?1 OR importance IS NULL OR importance != ?2);
            "#,
        )
        .bind(news_level)
        .bind(importance)
        .bind(existing.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Met a jour le niveau d'une ligne deja dedupliquee quand le doublon porte une information plus recente.
    pub(super) async fn merge_news_level_for_duplicate(
        &self,
        event: &NewsEvent,
    ) -> Result<(), AppError> {
        if event.source == "financial_juice" {
            self.promote_financial_juice_duplicate(event).await?;
        }

        if event.source == "financial_juice_browser" {
            return Ok(());
        }

        let Some(news_level) = event.news_level.as_deref() else {
            if event.source == "financial_juice" {
                self.fill_missing_financial_juice_importance_by_signature(&event.signature)
                    .await?;
            }
            // Sans niveau entrant, on preserve strictement la ligne existante.
            return Ok(());
        };
        let importance = financial_juice_importance_from_news_level(Some(news_level));

        // La signature identifie la news deja stockee; seule la colonne enrichissante est mergee.
        sqlx::query(
            r#"
            UPDATE news_events
            SET news_level = ?1,
                importance = ?2
            WHERE signature = ?3
              AND (news_level IS NULL OR news_level != ?1 OR importance IS NULL OR importance != ?2);
            "#,
        )
        .bind(news_level)
        .bind(importance)
        .bind(&event.signature)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Quand la source officielle arrive apres le fallback navigateur, elle redevient proprietaire.
    async fn promote_financial_juice_duplicate(&self, event: &NewsEvent) -> Result<(), AppError> {
        let existing = self.get_by_signature(&event.signature).await?;
        self.promote_financial_juice_duplicate_by_id(existing.id, event)
            .await
    }

    /// Variante par identifiant, utile quand le doublon est detecte par texte plutot que par signature.
    async fn promote_financial_juice_duplicate_by_id(
        &self,
        existing_id: i64,
        event: &NewsEvent,
    ) -> Result<(), AppError> {
        let Some(existing) = self.get_news_by_id(existing_id).await? else {
            return Ok(());
        };
        let effective_news_level = event
            .news_level
            .as_deref()
            .or(existing.news_level.as_deref());
        let importance = financial_juice_importance_from_news_level(effective_news_level);
        let result = sqlx::query(
            r#"
            UPDATE news_events
            SET source = ?1,
                event_kind = ?2,
                title = ?3,
                url = ?4,
                published_at = ?5,
                event_time = ?6,
                body = ?7,
                news_level = COALESCE(?8, news_level),
                importance = ?9,
                raw_payload_ref = ?10,
                ingested_at = ?11
            WHERE id = ?12
              AND source = 'financial_juice_browser';
            "#,
        )
        .bind(&event.source)
        .bind(event.event_kind.as_str())
        .bind(&event.title)
        .bind(&event.url)
        .bind(&event.published_at)
        .bind(&event.event_time)
        .bind(&event.body)
        .bind(&event.news_level)
        .bind(importance)
        .bind(&event.raw_payload_ref)
        .bind(&event.ingested_at)
        .bind(existing_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(());
        }

        sqlx::query(
            r#"
            UPDATE news_translations
            SET source = ?1,
                event_kind = ?2,
                source_title = ?3,
                source_body = ?4,
                published_at = ?5,
                updated_at = ?6
            WHERE news_event_id = ?7;
            "#,
        )
        .bind(&event.source)
        .bind(event.event_kind.as_str())
        .bind(&event.title)
        .bind(&event.body)
        .bind(&event.published_at)
        .bind(now_rfc3339())
        .bind(existing_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn fill_missing_financial_juice_importance_by_id(&self, id: i64) -> Result<(), AppError> {
        let Some(existing) = self.get_news_by_id(id).await? else {
            return Ok(());
        };
        let importance = financial_juice_importance_from_news_level(existing.news_level.as_deref());
        sqlx::query(
            r#"
            UPDATE news_events
            SET importance = ?1
            WHERE id = ?2
              AND source = 'financial_juice'
              AND (importance IS NULL OR trim(importance) = '');
            "#,
        )
        .bind(importance)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn fill_missing_financial_juice_importance_by_signature(
        &self,
        signature: &str,
    ) -> Result<(), AppError> {
        let existing = self.get_by_signature(signature).await?;
        self.fill_missing_financial_juice_importance_by_id(existing.id)
            .await
    }
}
