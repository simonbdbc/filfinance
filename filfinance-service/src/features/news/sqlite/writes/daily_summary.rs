//! Ecritures SQLite des resumes IA journaliers.
#![allow(dead_code)]

use crate::{
    errors::AppError,
    features::news::domain::{
        DailyNewsQualificationDraft, DailyNewsQualificationUpsert, DailySummaryDraft,
        DailySummaryRunDraft, PersistedDailyNewsQualification, PersistedDailySummary,
        PersistedDailySummaryRun, PersistedDailySummaryRunArtifact, PersistedDailySummaryRunStep,
    },
    shared::time::now_rfc3339,
};

use super::super::SqliteNewsStore;

const DAILY_SUMMARY_SELECT: &str = r#"
    SELECT
        id,
        local_day,
        content_hash,
        summary_kind,
        status,
        title,
        daily_context_text,
        important_news_summary_text,
        summary_text,
        narration_text,
        stream_id,
        playlist_url,
        news_count,
        important_news_count,
        source_urls_json,
        error_message,
        provider,
        model,
        agent_latency_ms,
        created_at,
        updated_at
    FROM daily_summaries
"#;

const DAILY_NEWS_QUALIFICATION_SELECT: &str = r#"
    SELECT
        id,
        local_day,
        news_event_id,
        translation_id,
        category,
        importance,
        summary,
        reasoning,
        confidence,
        is_important,
        provider,
        model,
        qualified_at,
        created_at,
        updated_at
    FROM daily_news_qualifications
"#;

const DAILY_SUMMARY_RUN_SELECT: &str = r#"
    SELECT
        id,
        run_id,
        local_day,
        summary_id,
        status,
        reason,
        forced,
        error_message,
        created_at,
        updated_at,
        completed_at
    FROM daily_summary_runs
"#;

const DAILY_SUMMARY_RUN_STEP_SELECT: &str = r#"
    SELECT
        id,
        run_id,
        local_day,
        stage,
        status,
        message,
        detail,
        created_at
    FROM daily_summary_run_steps
"#;

const DAILY_SUMMARY_RUN_ARTIFACT_SELECT: &str = r#"
    SELECT
        id,
        run_id,
        local_day,
        kind,
        label,
        content,
        created_at
    FROM daily_summary_run_artifacts
"#;

impl SqliteNewsStore {
    /// Retourne le resume exact d'un jour et d'un contenu source.
    pub async fn get_daily_summary_by_hash(
        &self,
        local_day: &str,
        content_hash: &str,
    ) -> Result<Option<PersistedDailySummary>, AppError> {
        let summary = sqlx::query_as::<_, PersistedDailySummary>(&format!(
            "{DAILY_SUMMARY_SELECT} WHERE local_day = ? AND content_hash = ? LIMIT 1"
        ))
        .bind(local_day)
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(summary)
    }

    /// Retourne un resume journalier par cle primaire.
    pub async fn get_daily_summary_by_id(
        &self,
        summary_id: i64,
    ) -> Result<Option<PersistedDailySummary>, AppError> {
        let summary = sqlx::query_as::<_, PersistedDailySummary>(&format!(
            "{DAILY_SUMMARY_SELECT} WHERE id = ? LIMIT 1"
        ))
        .bind(summary_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(summary)
    }

    /// Cree le run pending si absent, puis retourne la ligne existante ou creee.
    pub async fn ensure_daily_summary_pending(
        &self,
        draft: &DailySummaryDraft,
    ) -> Result<PersistedDailySummary, AppError> {
        let now = now_rfc3339();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO daily_summaries (
                local_day,
                content_hash,
                summary_kind,
                status,
                news_count,
                important_news_count,
                source_urls_json,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, 'pending', ?, ?, ?, ?, ?);
            "#,
        )
        .bind(&draft.local_day)
        .bind(&draft.content_hash)
        .bind(&draft.summary_kind)
        .bind(draft.news_count)
        .bind(draft.important_news_count)
        .bind(&draft.source_urls_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_daily_summary_by_hash(&draft.local_day, &draft.content_hash)
            .await?
            .ok_or_else(|| AppError::Storage("daily summary was not persisted".to_owned()))
    }

    /// Remet un run en attente quand le precedent etait en erreur.
    pub async fn mark_daily_summary_pending(&self, summary_id: i64) -> Result<(), AppError> {
        let now = now_rfc3339();
        sqlx::query(
            r#"
            UPDATE daily_summaries
            SET status = 'pending',
                title = NULL,
                daily_context_text = NULL,
                important_news_summary_text = NULL,
                summary_text = NULL,
                narration_text = NULL,
                stream_id = NULL,
                playlist_url = NULL,
                error_message = NULL,
                updated_at = ?
            WHERE id = ?;
            "#,
        )
        .bind(now)
        .bind(summary_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retourne la plus grande cle news connue par le backend.
    pub async fn max_news_event_id(&self) -> Result<Option<i64>, AppError> {
        let max_id = sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(id) FROM news_events;")
            .fetch_one(&self.pool)
            .await?;
        Ok(max_id)
    }

    /// Lit le watermark du scheduler de resume journalier.
    pub async fn get_daily_summary_scheduler_watermark(
        &self,
        state_key: &str,
    ) -> Result<Option<i64>, AppError> {
        let value = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT last_news_event_id
            FROM daily_summary_scheduler_state
            WHERE state_key = ?;
            "#,
        )
        .bind(state_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(value)
    }

    /// Met a jour le watermark du scheduler apres acceptation du Webhook n8n.
    pub async fn set_daily_summary_scheduler_watermark(
        &self,
        state_key: &str,
        last_news_event_id: i64,
    ) -> Result<(), AppError> {
        let now = now_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO daily_summary_scheduler_state (state_key, last_news_event_id, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT(state_key) DO UPDATE SET
                last_news_event_id = excluded.last_news_event_id,
                updated_at = excluded.updated_at;
            "#,
        )
        .bind(state_key)
        .bind(last_news_event_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Upsert idempotent d'une qualification de news du jour.
    pub async fn upsert_daily_news_qualification(
        &self,
        draft: &DailyNewsQualificationDraft,
    ) -> Result<DailyNewsQualificationUpsert, AppError> {
        let existing = self
            .get_daily_news_qualification(&draft.local_day, draft.news_event_id)
            .await?;
        let became_important = draft.is_important
            && existing
                .as_ref()
                .map(|qualification| !qualification.is_important)
                .unwrap_or(true);
        let now = now_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO daily_news_qualifications (
                local_day,
                news_event_id,
                translation_id,
                category,
                importance,
                summary,
                reasoning,
                confidence,
                is_important,
                provider,
                model,
                qualified_at,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(local_day, news_event_id) DO UPDATE SET
                translation_id = excluded.translation_id,
                category = excluded.category,
                importance = excluded.importance,
                summary = excluded.summary,
                reasoning = excluded.reasoning,
                confidence = excluded.confidence,
                is_important = excluded.is_important,
                provider = excluded.provider,
                model = excluded.model,
                qualified_at = excluded.qualified_at,
                updated_at = excluded.updated_at;
            "#,
        )
        .bind(&draft.local_day)
        .bind(draft.news_event_id)
        .bind(draft.translation_id)
        .bind(&draft.category)
        .bind(&draft.importance)
        .bind(&draft.summary)
        .bind(&draft.reasoning)
        .bind(draft.confidence)
        .bind(draft.is_important)
        .bind(&draft.provider)
        .bind(&draft.model)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let qualification = self
            .get_daily_news_qualification(&draft.local_day, draft.news_event_id)
            .await?
            .ok_or_else(|| {
                AppError::Storage("daily news qualification was not persisted".into())
            })?;

        Ok(DailyNewsQualificationUpsert {
            qualification,
            became_important,
        })
    }

    /// Liste toutes les qualifications stockees pour un jour donne.
    pub async fn list_daily_news_qualifications(
        &self,
        local_day: &str,
    ) -> Result<Vec<PersistedDailyNewsQualification>, AppError> {
        let items = sqlx::query_as::<_, PersistedDailyNewsQualification>(&format!(
            "{DAILY_NEWS_QUALIFICATION_SELECT} WHERE local_day = ? ORDER BY news_event_id ASC"
        ))
        .bind(local_day)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    /// Retourne une qualification precise.
    pub async fn get_daily_news_qualification(
        &self,
        local_day: &str,
        news_event_id: i64,
    ) -> Result<Option<PersistedDailyNewsQualification>, AppError> {
        let qualification = sqlx::query_as::<_, PersistedDailyNewsQualification>(&format!(
            "{DAILY_NEWS_QUALIFICATION_SELECT} WHERE local_day = ? AND news_event_id = ? LIMIT 1"
        ))
        .bind(local_day)
        .bind(news_event_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(qualification)
    }

    /// Liste les news importantes stockees pour un jour donne.
    pub async fn list_important_daily_news_qualifications(
        &self,
        local_day: &str,
    ) -> Result<Vec<PersistedDailyNewsQualification>, AppError> {
        let items = sqlx::query_as::<_, PersistedDailyNewsQualification>(
            r#"
            SELECT
                q.id,
                q.local_day,
                q.news_event_id,
                q.translation_id,
                q.category,
                q.importance,
                q.summary,
                q.reasoning,
                q.confidence,
                q.is_important,
                q.provider,
                q.model,
                q.qualified_at,
                q.created_at,
                q.updated_at
            FROM daily_news_qualifications q
            LEFT JOIN news_events e ON e.id = q.news_event_id
            WHERE q.local_day = ? AND q.is_important = 1
            ORDER BY COALESCE(e.published_at, e.ingested_at, q.qualified_at) ASC, q.news_event_id ASC;
            "#,
        )
        .bind(local_day)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    /// Persiste le texte produit par n8n avant generation audio.
    pub async fn mark_daily_summary_summarized(
        &self,
        summary_id: i64,
        title: &str,
        daily_context_text: Option<&str>,
        important_news_summary_text: Option<&str>,
        summary_text: &str,
        narration_text: &str,
        provider: Option<&str>,
        model: Option<&str>,
        agent_latency_ms: Option<i64>,
    ) -> Result<(), AppError> {
        let now = now_rfc3339();
        sqlx::query(
            r#"
            UPDATE daily_summaries
            SET status = 'audio_generating',
                title = ?,
                daily_context_text = ?,
                important_news_summary_text = ?,
                summary_text = ?,
                narration_text = ?,
                provider = ?,
                model = ?,
                agent_latency_ms = ?,
                error_message = NULL,
                updated_at = ?
            WHERE id = ?;
            "#,
        )
        .bind(title)
        .bind(daily_context_text)
        .bind(important_news_summary_text)
        .bind(summary_text)
        .bind(narration_text)
        .bind(provider)
        .bind(model)
        .bind(agent_latency_ms)
        .bind(now)
        .bind(summary_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Persiste la playlist HLS du resume journalier.
    pub async fn mark_daily_summary_audio_ready(
        &self,
        summary_id: i64,
        stream_id: &str,
        playlist_url: &str,
    ) -> Result<(), AppError> {
        let now = now_rfc3339();
        sqlx::query(
            r#"
            UPDATE daily_summaries
            SET status = 'audio_ready',
                stream_id = ?,
                playlist_url = ?,
                error_message = NULL,
                updated_at = ?
            WHERE id = ?;
            "#,
        )
        .bind(stream_id)
        .bind(playlist_url)
        .bind(now)
        .bind(summary_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Marque un resume en erreur sans bloquer le pipeline live.
    pub async fn mark_daily_summary_error(
        &self,
        summary_id: i64,
        error_message: &str,
    ) -> Result<(), AppError> {
        let now = now_rfc3339();
        sqlx::query(
            r#"
            UPDATE daily_summaries
            SET status = 'error',
                error_message = ?,
                updated_at = ?
            WHERE id = ?;
            "#,
        )
        .bind(error_message)
        .bind(now)
        .bind(summary_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Cree ou relit un run admin de resume journalier.
    pub async fn ensure_daily_summary_run(
        &self,
        draft: &DailySummaryRunDraft,
    ) -> Result<PersistedDailySummaryRun, AppError> {
        let now = now_rfc3339();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO daily_summary_runs (
                run_id,
                local_day,
                summary_id,
                status,
                reason,
                forced,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(&draft.run_id)
        .bind(&draft.local_day)
        .bind(draft.summary_id)
        .bind(&draft.status)
        .bind(&draft.reason)
        .bind(draft.forced)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_daily_summary_run(&draft.run_id)
            .await?
            .ok_or_else(|| AppError::Storage("daily summary run was not persisted".to_owned()))
    }

    /// Met a jour le statut d'un run admin.
    pub async fn update_daily_summary_run_status(
        &self,
        run_id: &str,
        status: &str,
        summary_id: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<Option<PersistedDailySummaryRun>, AppError> {
        let now = now_rfc3339();
        let completed_at = matches!(status, "audio_ready" | "error").then_some(now.as_str());
        sqlx::query(
            r#"
            UPDATE daily_summary_runs
            SET status = ?,
                summary_id = COALESCE(?, summary_id),
                error_message = ?,
                updated_at = ?,
                completed_at = COALESCE(?, completed_at)
            WHERE run_id = ?;
            "#,
        )
        .bind(status)
        .bind(summary_id)
        .bind(error_message)
        .bind(&now)
        .bind(completed_at)
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        self.get_daily_summary_run(run_id).await
    }

    /// Relit un run par identifiant stable.
    pub async fn get_daily_summary_run(
        &self,
        run_id: &str,
    ) -> Result<Option<PersistedDailySummaryRun>, AppError> {
        let item = sqlx::query_as::<_, PersistedDailySummaryRun>(&format!(
            "{DAILY_SUMMARY_RUN_SELECT} WHERE run_id = ? LIMIT 1"
        ))
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(item)
    }

    /// Retourne le dernier run d'un jour donne.
    pub async fn get_latest_daily_summary_run(
        &self,
        local_day: &str,
    ) -> Result<Option<PersistedDailySummaryRun>, AppError> {
        let item = sqlx::query_as::<_, PersistedDailySummaryRun>(&format!(
            "{DAILY_SUMMARY_RUN_SELECT} WHERE local_day = ? ORDER BY created_at DESC, id DESC LIMIT 1"
        ))
        .bind(local_day)
        .fetch_optional(&self.pool)
        .await?;

        Ok(item)
    }

    /// Ajoute une etape horodatee au run admin.
    pub async fn insert_daily_summary_run_step(
        &self,
        run_id: &str,
        local_day: &str,
        stage: &str,
        status: &str,
        message: &str,
        detail: Option<&str>,
    ) -> Result<PersistedDailySummaryRunStep, AppError> {
        let now = now_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO daily_summary_run_steps (
                run_id,
                local_day,
                stage,
                status,
                message,
                detail,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(run_id)
        .bind(local_day)
        .bind(stage)
        .bind(status)
        .bind(message)
        .bind(detail)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();
        sqlx::query_as::<_, PersistedDailySummaryRunStep>(&format!(
            "{DAILY_SUMMARY_RUN_STEP_SELECT} WHERE id = ? LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::Storage("daily summary run step was not persisted".to_owned()))
    }

    /// Liste les etapes d'un run dans l'ordre chronologique.
    pub async fn list_daily_summary_run_steps(
        &self,
        run_id: &str,
    ) -> Result<Vec<PersistedDailySummaryRunStep>, AppError> {
        let items = sqlx::query_as::<_, PersistedDailySummaryRunStep>(&format!(
            "{DAILY_SUMMARY_RUN_STEP_SELECT} WHERE run_id = ? ORDER BY created_at ASC, id ASC"
        ))
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }

    /// Ajoute un artifact redacte au run admin.
    pub async fn insert_daily_summary_run_artifact(
        &self,
        run_id: &str,
        local_day: &str,
        kind: &str,
        label: &str,
        content: &str,
    ) -> Result<PersistedDailySummaryRunArtifact, AppError> {
        let now = now_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO daily_summary_run_artifacts (
                run_id,
                local_day,
                kind,
                label,
                content,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?);
            "#,
        )
        .bind(run_id)
        .bind(local_day)
        .bind(kind)
        .bind(label)
        .bind(content)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();
        sqlx::query_as::<_, PersistedDailySummaryRunArtifact>(&format!(
            "{DAILY_SUMMARY_RUN_ARTIFACT_SELECT} WHERE id = ? LIMIT 1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::Storage("daily summary run artifact was not persisted".to_owned()))
    }

    /// Liste les artifacts d'un run dans l'ordre chronologique.
    pub async fn list_daily_summary_run_artifacts(
        &self,
        run_id: &str,
    ) -> Result<Vec<PersistedDailySummaryRunArtifact>, AppError> {
        let items = sqlx::query_as::<_, PersistedDailySummaryRunArtifact>(&format!(
            "{DAILY_SUMMARY_RUN_ARTIFACT_SELECT} WHERE run_id = ? ORDER BY created_at ASC, id ASC"
        ))
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }
}
