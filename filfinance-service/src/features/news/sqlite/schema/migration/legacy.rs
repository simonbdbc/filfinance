//! Colonnes legacy ajoutees aux anciennes bases news.

use crate::errors::AppError;

use super::super::super::SqliteNewsStore;

impl SqliteNewsStore {
    /// Ajoute les colonnes legacy de `news_events` si une base ancienne est ouverte.
    pub(super) async fn add_news_events_legacy_columns(&self) -> Result<(), AppError> {
        // `news_level` est arrivé après le MVP: les bases existantes doivent être enrichies sans rebuild.
        if !self.column_exists("news_events", "news_level").await? {
            sqlx::query(
                r#"
                ALTER TABLE news_events
                ADD COLUMN news_level TEXT;
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Ajoute les colonnes legacy de `news_translations` si une base ancienne est ouverte.
    pub(super) async fn add_news_translations_legacy_columns(&self) -> Result<(), AppError> {
        // Cette colonne a été ajoutée après coup; on la traite comme une migration "legacy-safe".
        if !self
            .column_exists("news_translations", "published_at")
            .await?
        {
            sqlx::query(
                r#"
                ALTER TABLE news_translations
                ADD COLUMN published_at TEXT;
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Ajoute les colonnes daily summary arrivees apres le premier MVP n8n.
    pub(super) async fn add_daily_summaries_legacy_columns(&self) -> Result<(), AppError> {
        if !self
            .column_exists("daily_summaries", "summary_kind")
            .await?
        {
            sqlx::query(
                r#"
                ALTER TABLE daily_summaries
                ADD COLUMN summary_kind TEXT NOT NULL DEFAULT 'ai';
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        if !self
            .column_exists("daily_summaries", "important_news_count")
            .await?
        {
            sqlx::query(
                r#"
                ALTER TABLE daily_summaries
                ADD COLUMN important_news_count INTEGER NOT NULL DEFAULT 0;
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        if !self
            .column_exists("daily_summaries", "daily_context_text")
            .await?
        {
            sqlx::query(
                r#"
                ALTER TABLE daily_summaries
                ADD COLUMN daily_context_text TEXT;
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        if !self
            .column_exists("daily_summaries", "important_news_summary_text")
            .await?
        {
            sqlx::query(
                r#"
                ALTER TABLE daily_summaries
                ADD COLUMN important_news_summary_text TEXT;
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Ajoute les colonnes glossaire arrivees apres le CRUD initial.
    pub(super) async fn add_translation_glossary_legacy_columns(&self) -> Result<(), AppError> {
        if !self
            .column_exists("translation_glossary_entries", "scope")
            .await?
        {
            sqlx::query(
                r#"
                ALTER TABLE translation_glossary_entries
                ADD COLUMN scope TEXT NOT NULL DEFAULT 'full_pipeline';
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Ajoute les colonnes blacklist arrivees avec la portee titre/contenu.
    pub(super) async fn add_news_blacklist_legacy_columns(&self) -> Result<(), AppError> {
        if !self
            .column_exists("news_blacklist_entries", "target_scope")
            .await?
        {
            sqlx::query(
                r#"
                ALTER TABLE news_blacklist_entries
                ADD COLUMN target_scope TEXT NOT NULL DEFAULT 'title_content';
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        if !self
            .column_exists("news_blacklist_hits", "target_scope")
            .await?
        {
            sqlx::query(
                r#"
                ALTER TABLE news_blacklist_hits
                ADD COLUMN target_scope TEXT NOT NULL DEFAULT 'title_content';
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        if !self.column_exists("news_blacklist_hits", "action").await? {
            sqlx::query(
                r#"
                ALTER TABLE news_blacklist_hits
                ADD COLUMN action TEXT NOT NULL DEFAULT 'reject_news';
                "#,
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Supprime les tables de benchmark audio qui ne sont plus utilisées par le runtime actif.
    pub(super) async fn drop_obsolete_news_audio_pipeline_candidates(
        &self,
    ) -> Result<(), AppError> {
        sqlx::query("DROP TABLE IF EXISTS news_audio_pipeline_candidates;")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Supprime l'ancienne table calendrier FinancialJuice devenue inactive.
    pub(super) async fn drop_obsolete_financial_juice_calendar_events(
        &self,
    ) -> Result<(), AppError> {
        sqlx::query("DROP TABLE IF EXISTS calendar_events;")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
