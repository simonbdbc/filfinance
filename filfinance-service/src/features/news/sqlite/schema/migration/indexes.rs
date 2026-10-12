//! Indexes de lecture SQLite de la feature news.

use crate::errors::AppError;

/// Crée les indexes de lecture utilisés par HTTP, traduction et WebSocket.
pub(super) async fn create_read_indexes(pool: &sqlx::SqlitePool) -> Result<(), AppError> {
    // Les indexes suivants servent des lectures "les plus récentes d'abord" sans scanner toute la table.
    // Les deux indexes couvrent les lectures récentes par source et par type d'événement.
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_source_ingested_at
        ON news_events (source, ingested_at DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_ingested_at
        ON news_events (event_kind, ingested_at DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    // Les indexes par `id DESC` servent la pagination `before_id` retenue pour le MVP.
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_source_id_desc
        ON news_events (source, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_id_desc
        ON news_events (event_kind, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_translations_language_id_desc
        ON news_translations (language, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_translations_news_event_id
        ON news_translations (news_event_id);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_published_at_desc
        ON news_events (COALESCE(published_at, ingested_at) DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_source_published_at_desc
        ON news_events (source, COALESCE(published_at, ingested_at) DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_published_at_desc
        ON news_events (event_kind, COALESCE(published_at, ingested_at) DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_visible_day
        ON news_events (substr(COALESCE(published_at, ingested_at), 1, 10), id);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_events_retention_day
        ON news_events (substr(COALESCE(published_at, event_time, ingested_at), 1, 10), id);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_translations_language_status_published_at_desc
        ON news_translations (language, status, COALESCE(published_at, created_at) DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_translations_retention_day
        ON news_translations (substr(COALESCE(published_at, created_at), 1, 10), id);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_daily_summaries_local_day_updated_at
        ON daily_summaries (local_day, updated_at DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_daily_news_qualifications_local_day_important
        ON daily_news_qualifications (local_day, is_important, updated_at DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_daily_news_qualifications_news_event_id
        ON daily_news_qualifications (news_event_id);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_daily_summary_runs_local_day_created_at
        ON daily_summary_runs (local_day, created_at DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_daily_summary_run_steps_run_id_created_at
        ON daily_summary_run_steps (run_id, created_at ASC, id ASC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_daily_summary_run_artifacts_run_id_created_at
        ON daily_summary_run_artifacts (run_id, created_at ASC, id ASC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_translation_glossary_entries_enabled_priority
        ON translation_glossary_entries (enabled, priority DESC, source_term);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_translation_glossary_entries_source_term
        ON translation_glossary_entries (source_term);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_blacklist_entries_enabled_term
        ON news_blacklist_entries (enabled, term);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_blacklist_hits_created_at
        ON news_blacklist_hits (created_at DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_news_truncation_audit_created_at
        ON news_truncation_audit (created_at DESC, id DESC);
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
