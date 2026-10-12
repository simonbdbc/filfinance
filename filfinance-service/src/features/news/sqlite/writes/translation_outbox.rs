//! Admission atomique des événements et travaux de traduction durables.

use sqlx::{Sqlite, Transaction};

use crate::{
    errors::AppError,
    features::{
        news::{
            domain::{NewsEvent, PersistedNewsEvent, financial_juice_importance_from_news_level},
            sqlite::{SqliteNewsStore, statements::NEWS_EVENT_SELECT},
        },
        translation::domain::{
            NewTranslationWork, NewsTranslationAdmission, PersistedTranslationWork,
            TranslationWorkStatus,
        },
    },
    shared::time::now_rfc3339,
};

const TRANSLATION_OUTBOX_SELECT: &str = r#"
    SELECT
        id,
        news_event_id,
        target_language,
        status,
        attempt_count,
        available_at,
        lease_generation,
        lease_expires_at,
        last_error_code,
        created_at,
        updated_at,
        completed_at
    FROM news_translation_outbox
"#;

#[derive(Debug, sqlx::FromRow)]
struct TranslationOutboxRow {
    id: i64,
    news_event_id: i64,
    target_language: String,
    status: String,
    attempt_count: i64,
    available_at: String,
    lease_generation: i64,
    lease_expires_at: Option<String>,
    last_error_code: Option<String>,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
}

impl TryFrom<TranslationOutboxRow> for PersistedTranslationWork {
    type Error = AppError;

    fn try_from(row: TranslationOutboxRow) -> Result<Self, Self::Error> {
        let attempt_count = u32::try_from(row.attempt_count)
            .map_err(|_| AppError::storage("news_translation_outbox has an invalid attempt count"))?;
        let lease_generation = u64::try_from(row.lease_generation).map_err(|_| {
            AppError::storage("news_translation_outbox has an invalid lease generation")
        })?;

        Ok(Self {
            id: row.id,
            news_event_id: row.news_event_id,
            target_language: row.target_language,
            status: TranslationWorkStatus::from_storage(&row.status)?,
            attempt_count,
            available_at: row.available_at,
            lease_generation,
            lease_expires_at: row.lease_expires_at,
            last_error_code: row.last_error_code,
            created_at: row.created_at,
            updated_at: row.updated_at,
            completed_at: row.completed_at,
        })
    }
}

impl SqliteNewsStore {
    /// Admet une nouvelle news et son travail de traduction dans une transaction unique.
    ///
    /// Cette API est délibérément dormante : le pipeline live conserve encore
    /// `insert_event -> broadcast` jusqu'à la bascule complète du consumer SQLite.
    /// Elle fournit toutefois la transaction qui empêchera un événement brut
    /// committé sans travail durable lorsque cette bascule sera autorisée.
    ///
    /// La déduplication textuelle FinancialJuice est encore celle du pipeline
    /// legacy. Cette API ne doit donc être raccordée qu'après mise en place du
    /// writer unique/lease inter-processus de l'ingestion ; elle ne prétend pas
    /// sérialiser plusieurs producteurs dans ce palier dormant.
    pub(in crate::features::news) async fn admit_event_for_translation(
        &self,
        event: &NewsEvent,
        work: &NewTranslationWork,
    ) -> Result<NewsTranslationAdmission, AppError> {
        // Les doublons texte FinancialJuice gardent strictement leur comportement legacy.
        // Ils ne créent pas un second travail, car l'event déjà canonique conserve son ID.
        if let Some(existing_duplicate) = self
            .find_recent_financial_juice_text_duplicate(event)
            .await?
        {
            self.merge_financial_juice_text_duplicate(&existing_duplicate, event)
                .await?;
            let (persisted_event, persisted_work, work_inserted) = self
                .ensure_translation_work_for_existing_event(existing_duplicate.id, work)
                .await?;
            return Ok(NewsTranslationAdmission::Existing {
                event: persisted_event,
                work: persisted_work,
                work_inserted,
            });
        }

        let macro_metadata = event.macro_metadata.as_ref();
        let importance = if event.source == "financial_juice" {
            Some(financial_juice_importance_from_news_level(event.news_level.as_deref()).to_owned())
        } else {
            macro_metadata.and_then(|metadata| metadata.importance.clone())
        };

        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query(
            r#"
            INSERT INTO news_events (
                source,
                event_kind,
                title,
                url,
                published_at,
                event_time,
                ingested_at,
                body,
                news_level,
                importance,
                country_code,
                currency,
                actual_value,
                forecast_value,
                previous_value,
                raw_payload_ref,
                signature
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(signature) DO NOTHING;
            "#,
        )
        .bind(&event.source)
        .bind(event.event_kind.as_str())
        .bind(&event.title)
        .bind(&event.url)
        .bind(&event.published_at)
        .bind(&event.event_time)
        .bind(&event.ingested_at)
        .bind(&event.body)
        .bind(&event.news_level)
        .bind(importance)
        .bind(macro_metadata.and_then(|metadata| metadata.country_code.clone()))
        .bind(macro_metadata.and_then(|metadata| metadata.currency.clone()))
        .bind(macro_metadata.and_then(|metadata| metadata.actual_value.clone()))
        .bind(macro_metadata.and_then(|metadata| metadata.forecast_value.clone()))
        .bind(macro_metadata.and_then(|metadata| metadata.previous_value.clone()))
        .bind(&event.raw_payload_ref)
        .bind(&event.signature)
        .execute(&mut *tx)
        .await?;

        if inserted.rows_affected() == 0 {
            let event_select = format!("{NEWS_EVENT_SELECT} WHERE signature = ?1 LIMIT 1");
            let persisted_event = sqlx::query_as::<_, PersistedNewsEvent>(&event_select)
                .bind(&event.signature)
                .fetch_one(&mut *tx)
                .await?;
            let (persisted_work, work_inserted) =
                Self::ensure_translation_work_in_transaction(&mut tx, persisted_event.id, work)
                    .await?;

            // On termine la transaction avant de réutiliser le pool dans le merge legacy,
            // indispensable aux tests SQLite mono-connexion. Le travail canonique a été
            // créé ou relu dans cette même transaction que la relecture de l'événement.
            tx.commit().await?;
            self.merge_news_level_for_duplicate(event).await?;
            return Ok(NewsTranslationAdmission::Existing {
                event: persisted_event,
                work: persisted_work,
                work_inserted,
            });
        }

        // La relecture reste dans la transaction : une requête via le pool serait invisible
        // avant commit et pourrait attendre une autre connexion SQLite inutilement.
        let event_select = format!("{NEWS_EVENT_SELECT} WHERE signature = ?1 LIMIT 1");
        let persisted_event = sqlx::query_as::<_, PersistedNewsEvent>(&event_select)
            .bind(&event.signature)
            .fetch_one(&mut *tx)
            .await?;

        let (persisted_work, work_inserted) =
            Self::ensure_translation_work_in_transaction(&mut tx, persisted_event.id, work).await?;

        if !work_inserted {
            return Err(AppError::storage(
                "new news event is missing its translation outbox work",
            ));
        }

        tx.commit().await?;

        Ok(NewsTranslationAdmission::Inserted {
            event: persisted_event,
            work: persisted_work,
        })
    }

    async fn ensure_translation_work_for_existing_event(
        &self,
        news_event_id: i64,
        work: &NewTranslationWork,
    ) -> Result<(PersistedNewsEvent, PersistedTranslationWork, bool), AppError> {
        let mut tx = self.pool.begin().await?;
        let event_select = format!("{NEWS_EVENT_SELECT} WHERE id = ?1 LIMIT 1");
        let persisted_event = sqlx::query_as::<_, PersistedNewsEvent>(&event_select)
            .bind(news_event_id)
            .fetch_one(&mut *tx)
            .await?;
        let (persisted_work, work_inserted) =
            Self::ensure_translation_work_in_transaction(&mut tx, news_event_id, work).await?;
        tx.commit().await?;
        Ok((persisted_event, persisted_work, work_inserted))
    }

    async fn ensure_translation_work_in_transaction(
        tx: &mut Transaction<'_, Sqlite>,
        news_event_id: i64,
        work: &NewTranslationWork,
    ) -> Result<(PersistedTranslationWork, bool), AppError> {
        let timestamp = now_rfc3339();
        let inserted = sqlx::query(
            r#"
            INSERT INTO news_translation_outbox (
                news_event_id,
                target_language,
                status,
                attempt_count,
                available_at,
                lease_generation,
                lease_expires_at,
                last_error_code,
                created_at,
                updated_at,
                completed_at
            ) VALUES (?1, ?2, ?3, 0, ?4, 0, NULL, NULL, ?4, ?4, NULL)
            ON CONFLICT(news_event_id, target_language) DO NOTHING;
            "#,
        )
        .bind(news_event_id)
        .bind(work.target_language())
        .bind(TranslationWorkStatus::Pending.as_str())
        .bind(&timestamp)
        .execute(&mut **tx)
        .await?;

        let work_select = format!(
            "{TRANSLATION_OUTBOX_SELECT} WHERE news_event_id = ?1 AND target_language = ?2 LIMIT 1"
        );
        let persisted_work = sqlx::query_as::<_, TranslationOutboxRow>(&work_select)
            .bind(news_event_id)
            .bind(work.target_language())
            .fetch_one(&mut **tx)
            .await?
            .try_into()?;

        Ok((persisted_work, inserted.rows_affected() == 1))
    }
}
