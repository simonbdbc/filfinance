//! Enregistrement idempotent des migrations historiques news.

use crate::errors::AppError;

/// Enregistre les migrations historiques de manière idempotente.
pub(super) async fn record_schema_migrations(pool: &sqlx::SqlitePool) -> Result<(), AppError> {
    // Les noms de migration gardent un historique lisible des évolutions de schéma du MVP.
    // L'enregistrement de migration reste idempotent pour supporter les redémarrages.
    for migration_name in [
        "0001_init_news_events",
        "0002_add_read_indexes",
        "0003_add_news_translations",
        "0004_sort_news_by_published_at",
        "0006_add_news_level",
        "0011_add_daily_summaries",
        "0012_add_daily_news_qualifications",
        "0013_add_daily_summary_scheduler_state",
        "0014_drop_news_audio_pipeline_candidates",
        "0015_add_translation_glossary_entries",
        "0016_add_daily_summary_admin_runs",
        "0017_add_calendar_day_tables",
        "0018_add_news_blacklist_and_truncation_audit",
        "0020_drop_legacy_financial_juice_calendar_events",
        "0022_seed_translation_glossary_patterns",
        "0023_add_news_blacklist_target_scope",
        "0024_add_retention_and_feed_query_indexes",
        "0025_add_translation_glossary_scope",
        "0026_seed_spoken_time_glossary_patterns",
    ] {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO schema_migrations (name)
            VALUES (?);
            "#,
        )
        .bind(migration_name)
        .execute(pool)
        .await?;
    }

    Ok(())
}
