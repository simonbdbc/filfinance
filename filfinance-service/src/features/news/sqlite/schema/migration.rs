//! Facade des migrations idempotentes SQLite de la feature news.

mod history;
mod indexes;
mod legacy;
mod pragmas;
mod tables;

use crate::errors::AppError;

use super::super::SqliteNewsStore;

impl SqliteNewsStore {
    /// Initialise le schema minimal et les pragmas indispensables au mode WAL.
    pub async fn migrate(&self) -> Result<(), AppError> {
        pragmas::apply_sqlite_pragmas(&self.pool).await?;
        tables::create_schema_migrations_table(&self.pool).await?;
        tables::create_news_events_table(&self.pool).await?;
        self.add_news_events_legacy_columns().await?;
        tables::create_news_translations_table(&self.pool).await?;
        self.add_news_translations_legacy_columns().await?;
        tables::create_daily_summaries_table(&self.pool).await?;
        self.add_daily_summaries_legacy_columns().await?;
        tables::create_daily_news_qualifications_table(&self.pool).await?;
        tables::create_daily_summary_scheduler_state_table(&self.pool).await?;
        tables::create_daily_summary_runs_table(&self.pool).await?;
        tables::create_daily_summary_run_steps_table(&self.pool).await?;
        tables::create_daily_summary_run_artifacts_table(&self.pool).await?;
        tables::create_translation_glossary_entries_table(&self.pool).await?;
        self.add_translation_glossary_legacy_columns().await?;
        tables::seed_translation_glossary_pattern_entries(&self.pool).await?;
        tables::create_news_blacklist_entries_table(&self.pool).await?;
        tables::create_news_blacklist_hits_table(&self.pool).await?;
        self.add_news_blacklist_legacy_columns().await?;
        tables::create_news_truncation_audit_table(&self.pool).await?;
        self.drop_obsolete_news_audio_pipeline_candidates().await?;
        self.drop_obsolete_financial_juice_calendar_events().await?;
        indexes::create_read_indexes(&self.pool).await?;
        history::record_schema_migrations(&self.pool).await?;
        Ok(())
    }
}
