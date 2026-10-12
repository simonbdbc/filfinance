//! Retention SQLite des news persistantes.

use sqlx::{QueryBuilder, Sqlite};
use time::{Date, OffsetDateTime, Weekday};

use super::SqliteNewsStore;
use crate::{errors::AppError, shared::time::europe_paris_day_for_utc};

/// Rapport de purge de retention des news persistées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsRetentionPruneReport {
    /// Plus vieux jour conserve au format `YYYY-MM-DD`.
    pub oldest_retained_day: String,
    /// Plus vieux jour conserve pour les audits append-only au format `YYYY-MM-DD`.
    pub oldest_audit_retained_day: String,
    /// Retention effective des audits append-only exprimee en jours calendaires.
    pub data_audit_retention_days: usize,
    /// Nombre de qualifications journalieres supprimees.
    pub deleted_daily_news_qualifications: u64,
    /// Nombre de resumes journaliers supprimes.
    pub deleted_daily_summaries: u64,
    /// Nombre d'artifacts de runs legacy supprimes.
    pub deleted_daily_summary_run_artifacts: u64,
    /// Nombre d'etapes de runs legacy supprimees.
    pub deleted_daily_summary_run_steps: u64,
    /// Nombre de runs legacy supprimes.
    pub deleted_daily_summary_runs: u64,
    /// Nombre de traductions supprimees avant la suppression des news parentes.
    pub deleted_translations: u64,
    /// Nombre de news supprimees dans `news_events`.
    pub deleted_news_events: u64,
    /// Nombre d'audits de hits blacklist supprimes.
    pub deleted_news_blacklist_hits: u64,
    /// Nombre d'audits de troncature upstream supprimes.
    pub deleted_news_truncation_audits: u64,
    /// Nombre de lignes calendrier supprimees.
    pub deleted_calendar_rows: u64,
    /// Jours calendrier ouvres conserves au format `YYYY-MM-DD`.
    pub retained_calendar_days: Vec<String>,
}

impl SqliteNewsStore {
    /// Supprime les news plus anciennes que la journée UTC courante et J-1.
    pub async fn prune_news_retention(
        &self,
        now_utc: OffsetDateTime,
        data_audit_retention_days: usize,
    ) -> Result<NewsRetentionPruneReport, AppError> {
        // Le jour J-1 est la borne inclusive conservee: toute date strictement inferieure est J-2+.
        let oldest_retained_day = previous_day_or_same(now_utc.date()).to_string();
        let data_audit_retention_days = data_audit_retention_days.max(1);
        let oldest_audit_retained_day =
            oldest_retained_day_for_retention(now_utc.date(), data_audit_retention_days)
                .to_string();
        let oldest_audit_retained_at = format!("{oldest_audit_retained_day}T00:00:00Z");

        let deleted_daily_news_qualifications = sqlx::query(
            r#"
            DELETE FROM daily_news_qualifications
            WHERE local_day < ?1
               OR news_event_id IN (
                SELECT id
                FROM news_events
                WHERE substr(COALESCE(published_at, event_time, ingested_at), 1, 10) < ?1
            )
               OR NOT EXISTS (
                SELECT 1
                FROM news_events
                WHERE news_events.id = daily_news_qualifications.news_event_id
            )
            "#,
        )
        .bind(&oldest_retained_day)
        .execute(&self.pool)
        .await?
        .rows_affected();

        let deleted_daily_summaries = sqlx::query(
            r#"
            DELETE FROM daily_summaries
            WHERE local_day < ?1
            "#,
        )
        .bind(&oldest_retained_day)
        .execute(&self.pool)
        .await?
        .rows_affected();

        let deleted_daily_summary_run_artifacts = delete_daily_summary_child_rows(
            &self.pool,
            "daily_summary_run_artifacts",
            &oldest_retained_day,
        )
        .await?;
        let deleted_daily_summary_run_steps = delete_daily_summary_child_rows(
            &self.pool,
            "daily_summary_run_steps",
            &oldest_retained_day,
        )
        .await?;
        let deleted_daily_summary_runs = sqlx::query(
            r#"
            DELETE FROM daily_summary_runs
            WHERE local_day < ?1
            "#,
        )
        .bind(&oldest_retained_day)
        .execute(&self.pool)
        .await?
        .rows_affected();

        // Les traductions sont purgees avant les news pour compenser l'absence de cascade SQLite.
        // On couvre aussi les lignes orphelines ou datees J-2+ par leur propre timestamp.
        let deleted_translations = sqlx::query(
            r#"
            DELETE FROM news_translations
            WHERE substr(COALESCE(news_translations.published_at, news_translations.created_at), 1, 10) < ?1
               OR news_event_id IN (
                SELECT id
                FROM news_events
                WHERE substr(COALESCE(published_at, event_time, ingested_at), 1, 10) < ?1
            )
               OR NOT EXISTS (
                SELECT 1
                FROM news_events
                WHERE news_events.id = news_translations.news_event_id
            )
            "#,
        )
        .bind(&oldest_retained_day)
        .execute(&self.pool)
        .await?
        .rows_affected();

        let deleted_news_events = sqlx::query(
            r#"
            DELETE FROM news_events
            WHERE substr(COALESCE(published_at, event_time, ingested_at), 1, 10) < ?1
            "#,
        )
        .bind(&oldest_retained_day)
        .execute(&self.pool)
        .await?
        .rows_affected();

        let retained_calendar_days = retained_calendar_business_days(now_utc);
        let deleted_calendar_rows = self
            .prune_calendar_retention(&retained_calendar_days)
            .await?;

        let deleted_news_blacklist_hits = sqlx::query(
            r#"
            DELETE FROM news_blacklist_hits
            WHERE created_at < ?1
            "#,
        )
        .bind(&oldest_audit_retained_at)
        .execute(&self.pool)
        .await?
        .rows_affected();

        let deleted_news_truncation_audits = sqlx::query(
            r#"
            DELETE FROM news_truncation_audit
            WHERE created_at < ?1
            "#,
        )
        .bind(&oldest_audit_retained_at)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(NewsRetentionPruneReport {
            oldest_retained_day,
            oldest_audit_retained_day,
            data_audit_retention_days,
            deleted_daily_news_qualifications,
            deleted_daily_summaries,
            deleted_daily_summary_run_artifacts,
            deleted_daily_summary_run_steps,
            deleted_daily_summary_runs,
            deleted_translations,
            deleted_news_events,
            deleted_news_blacklist_hits,
            deleted_news_truncation_audits,
            deleted_calendar_rows,
            retained_calendar_days,
        })
    }

    async fn prune_calendar_retention(&self, retained_days: &[String]) -> Result<u64, AppError> {
        if retained_days.is_empty() {
            return Ok(0);
        }
        if !calendar_retention_tables_exist(&self.pool).await? {
            return Ok(0);
        }

        let mut deleted = 0_u64;

        deleted += delete_calendar_child_rows_without_retained_items(
            &self.pool,
            "calendar_day_item_sources",
            retained_days,
        )
        .await?;
        deleted += delete_calendar_child_rows_without_retained_items(
            &self.pool,
            "calendar_day_item_audio",
            retained_days,
        )
        .await?;
        if calendar_table_exists(&self.pool, "calendar_day_source_audits").await? {
            deleted += delete_calendar_rows_not_in(
                &self.pool,
                "calendar_day_source_audits",
                "local_day",
                retained_days,
                "",
            )
            .await?;
        }
        // Les intentions encore pending/leased sont conservées quel que soit
        // leur jour afin qu'un redemarrage puisse les reprendre. Seuls les
        // travaux terminalement traites sortent avec la retention calendrier.
        if calendar_table_exists(&self.pool, "calendar_native_work").await? {
            deleted += delete_calendar_rows_not_in(
                &self.pool,
                "calendar_native_work",
                "local_day",
                retained_days,
                " AND status IN ('completed', 'dead_letter')",
            )
            .await?;
        }
        deleted += delete_calendar_rows_not_in(
            &self.pool,
            "calendar_day_alerts",
            "local_day",
            retained_days,
            "",
        )
        .await?;
        deleted += delete_calendar_rows_not_in(
            &self.pool,
            "calendar_day_items",
            "local_day",
            retained_days,
            "",
        )
        .await?;
        deleted += delete_calendar_rows_not_in(
            &self.pool,
            "calendar_day_summaries",
            "local_day",
            retained_days,
            "",
        )
        .await?;
        deleted += delete_calendar_rows_not_in(
            &self.pool,
            "calendar_day_run_steps",
            "local_day",
            retained_days,
            "",
        )
        .await?;
        deleted += delete_calendar_rows_not_in(
            &self.pool,
            "calendar_day_run_artifacts",
            "local_day",
            retained_days,
            "",
        )
        .await?;
        deleted += delete_calendar_rows_not_in(
            &self.pool,
            "calendar_day_runs",
            "local_day",
            retained_days,
            "",
        )
        .await?;
        deleted += delete_calendar_rows_not_in(
            &self.pool,
            "calendar_day_state",
            "local_day",
            retained_days,
            "",
        )
        .await?;

        Ok(deleted)
    }
}

async fn calendar_retention_tables_exist(pool: &sqlx::SqlitePool) -> Result<bool, AppError> {
    calendar_table_exists(pool, "calendar_day_state").await
}

/// Les migrations calendaires sont additives. La retention tolere donc une
/// base historique ouverte une seule fois avant l'ajout de la nouvelle queue.
async fn calendar_table_exists(
    pool: &sqlx::SqlitePool,
    table_name: &str,
) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table'
          AND name = ?1
        "#,
    )
    .bind(table_name)
    .fetch_one(pool)
    .await?;
    Ok(exists > 0)
}

/// Retourne le jour precedent quand il existe, sinon conserve le jour fourni.
fn previous_day_or_same(date: Date) -> Date {
    date.previous_day().unwrap_or(date)
}

/// Retourne le premier jour conserve pour une retention inclusive de `retention_days`.
fn oldest_retained_day_for_retention(date: Date, retention_days: usize) -> Date {
    let mut retained_day = date;
    for _ in 1..retention_days {
        retained_day = retained_day.previous_day().unwrap_or(retained_day);
    }
    retained_day
}

async fn delete_daily_summary_child_rows(
    pool: &sqlx::SqlitePool,
    table_name: &str,
    oldest_retained_day: &str,
) -> Result<u64, AppError> {
    let mut query =
        QueryBuilder::<Sqlite>::new(format!("DELETE FROM {table_name} WHERE local_day < "));
    query.push_bind(oldest_retained_day);
    query.push(" OR run_id IN (SELECT run_id FROM daily_summary_runs WHERE local_day < ");
    query.push_bind(oldest_retained_day);
    query.push(
        ") OR NOT EXISTS (SELECT 1 FROM daily_summary_runs WHERE daily_summary_runs.run_id = ",
    );
    query.push(table_name);
    query.push(".run_id)");
    Ok(query.build().execute(pool).await?.rows_affected())
}

async fn delete_calendar_rows_not_in(
    pool: &sqlx::SqlitePool,
    table_name: &str,
    column_expression: &str,
    retained_days: &[String],
    suffix: &str,
) -> Result<u64, AppError> {
    let mut query = QueryBuilder::<Sqlite>::new(format!(
        "DELETE FROM {table_name} WHERE {column_expression} NOT IN ("
    ));
    let mut separated = query.separated(", ");
    for day in retained_days {
        separated.push_bind(day);
    }
    query.push(")");
    query.push(suffix);
    Ok(query.build().execute(pool).await?.rows_affected())
}

async fn delete_calendar_child_rows_without_retained_items(
    pool: &sqlx::SqlitePool,
    table_name: &str,
    retained_days: &[String],
) -> Result<u64, AppError> {
    let mut query = QueryBuilder::<Sqlite>::new(format!(
        "DELETE FROM {table_name} WHERE calendar_item_id NOT IN (SELECT id FROM calendar_day_items WHERE local_day IN ("
    ));
    let mut separated = query.separated(", ");
    for day in retained_days {
        separated.push_bind(day);
    }
    query.push("))");
    Ok(query.build().execute(pool).await?.rows_affected())
}

fn retained_calendar_business_days(now_utc: OffsetDateTime) -> Vec<String> {
    let today = europe_paris_day_for_utc(now_utc).unwrap_or_else(|| now_utc.date());
    let mut days = business_week_days_for_date(today);
    if matches!(today.weekday(), Weekday::Saturday | Weekday::Sunday) {
        if let Some(next_monday) = add_days(monday_for_week(today), 7) {
            days.extend(business_week_days_for_date(next_monday));
        }
    }
    days.sort();
    days.dedup();
    days.into_iter().map(|day| day.to_string()).collect()
}

fn business_week_days_for_date(date: Date) -> Vec<Date> {
    let monday = monday_for_week(date);
    (0..5)
        .filter_map(|offset| add_days(monday, offset))
        .collect()
}

fn monday_for_week(date: Date) -> Date {
    let mut current = date;
    for _ in 0..weekday_index_from_monday(date.weekday()) {
        current = current.previous_day().unwrap_or(current);
    }
    current
}

fn add_days(mut date: Date, days: u8) -> Option<Date> {
    for _ in 0..days {
        date = date.next_day()?;
    }
    Some(date)
}

fn weekday_index_from_monday(weekday: Weekday) -> u8 {
    match weekday {
        Weekday::Monday => 0,
        Weekday::Tuesday => 1,
        Weekday::Wednesday => 2,
        Weekday::Thursday => 3,
        Weekday::Friday => 4,
        Weekday::Saturday => 5,
        Weekday::Sunday => 6,
    }
}

#[cfg(test)]
mod tests {
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    use crate::{
        config::StorageConfig,
        features::{calendar_day::CalendarDayStore, news::sqlite::SqliteNewsStore},
    };

    #[tokio::test]
    async fn retention_prunes_stale_and_orphan_translations_even_when_news_is_kept() {
        // Cette regression verrouille la règle produit stricte: aucune traduction J-2+ ou orpheline.
        let storage = test_storage().await;

        sqlx::query(
            r#"
            INSERT INTO news_events (
                id,
                source,
                event_kind,
                title,
                url,
                published_at,
                event_time,
                ingested_at,
                signature
            ) VALUES (1, 'financial_juice', 'headline', 'Today', 'https://example.test/today',
                '2026-05-05T08:00:00Z', '2026-05-05T08:00:00Z', '2026-05-05T08:00:01Z',
                'today-signature')
            "#,
        )
        .execute(&storage.pool)
        .await
        .expect("news fixture should insert");

        insert_translation_row(
            &storage,
            1,
            "fr",
            "2026-05-03T08:00:00Z",
            "2026-05-03T08:00:01Z",
        )
        .await;
        insert_translation_row(
            &storage,
            999,
            "en",
            "2026-05-05T08:00:00Z",
            "2026-05-05T08:00:01Z",
        )
        .await;

        let now = OffsetDateTime::parse("2026-05-05T12:00:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");
        let report = storage
            .prune_news_retention(now, 7)
            .await
            .expect("retention should prune translations");

        assert_eq!(report.oldest_retained_day, "2026-05-04");
        assert_eq!(report.deleted_news_events, 0);
        assert_eq!(report.deleted_translations, 2);
        assert_eq!(count_rows(&storage, "news_events").await, 1);
        assert_eq!(count_rows(&storage, "news_translations").await, 0);
    }

    #[tokio::test]
    async fn retention_keeps_news_j_j_minus_1_and_calendar_active_business_week() {
        let storage = test_storage_with_calendar().await;
        insert_calendar_state(&storage, "2026-05-01").await;
        insert_calendar_state(&storage, "2026-05-05").await;
        insert_calendar_state(&storage, "2026-05-12").await;

        let now = OffsetDateTime::parse("2026-05-05T12:00:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");
        let report = storage
            .prune_news_retention(now, 7)
            .await
            .expect("retention should prune calendar rows");

        assert_eq!(report.oldest_retained_day, "2026-05-04");
        assert_eq!(report.deleted_news_events, 0);
        assert_eq!(report.deleted_calendar_rows, 2);
        assert!(!calendar_state_exists(&storage, "2026-05-01").await);
        assert!(calendar_state_exists(&storage, "2026-05-05").await);
        assert!(!calendar_state_exists(&storage, "2026-05-12").await);
    }

    #[tokio::test]
    async fn weekend_retention_keeps_pregenerated_next_business_week() {
        let storage = test_storage_with_calendar().await;
        insert_calendar_state(&storage, "2026-05-05").await;
        insert_calendar_state(&storage, "2026-05-12").await;
        insert_calendar_state(&storage, "2026-05-18").await;

        let saturday = OffsetDateTime::parse("2026-05-09T12:00:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");
        let report = storage
            .prune_news_retention(saturday, 7)
            .await
            .expect("retention should keep next business week on weekends");

        assert_eq!(report.oldest_retained_day, "2026-05-08");
        assert_eq!(report.deleted_calendar_rows, 1);
        assert!(calendar_state_exists(&storage, "2026-05-05").await);
        assert!(calendar_state_exists(&storage, "2026-05-12").await);
        assert!(!calendar_state_exists(&storage, "2026-05-18").await);
    }

    #[tokio::test]
    async fn retention_prunes_legacy_daily_summary_run_tables_child_first() {
        let storage = test_storage().await;
        insert_daily_summary_run(&storage, "old-run", "2026-05-03").await;
        insert_daily_summary_run_step(&storage, "old-run", "2026-05-03").await;
        insert_daily_summary_run_artifact(&storage, "old-run", "2026-05-03").await;
        insert_daily_summary_run(&storage, "kept-run", "2026-05-04").await;
        insert_daily_summary_run_step(&storage, "kept-run", "2026-05-04").await;
        insert_daily_summary_run_artifact(&storage, "kept-run", "2026-05-04").await;
        insert_daily_summary_run_step(&storage, "orphan-run", "2026-05-05").await;
        insert_daily_summary_run_artifact(&storage, "orphan-run", "2026-05-05").await;

        let now = OffsetDateTime::parse("2026-05-05T12:00:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");
        let report = storage
            .prune_news_retention(now, 7)
            .await
            .expect("retention should prune legacy run rows");

        assert_eq!(report.oldest_retained_day, "2026-05-04");
        assert_eq!(report.deleted_daily_summary_run_artifacts, 2);
        assert_eq!(report.deleted_daily_summary_run_steps, 2);
        assert_eq!(report.deleted_daily_summary_runs, 1);
        assert!(!row_exists_by_run_id(&storage, "daily_summary_runs", "old-run").await);
        assert!(row_exists_by_run_id(&storage, "daily_summary_runs", "kept-run").await);
        assert!(!row_exists_by_run_id(&storage, "daily_summary_run_steps", "orphan-run").await);
        assert!(!row_exists_by_run_id(&storage, "daily_summary_run_artifacts", "orphan-run").await);
    }

    #[tokio::test]
    async fn retention_prunes_append_only_audits_without_touching_reference_tables() {
        let storage = test_storage().await;
        insert_news_blacklist_entry(&storage, "legacy blocked term", "2026-04-01T00:00:00Z").await;
        insert_daily_summary_scheduler_state(&storage, "legacy-summary", "2026-04-01T00:00:00Z")
            .await;
        insert_news_blacklist_hit(&storage, "old-hit", "2026-05-03T23:59:59Z").await;
        insert_news_blacklist_hit(&storage, "kept-hit", "2026-05-04T00:00:00Z").await;
        insert_news_truncation_audit(&storage, "old-audit", "2026-05-03T23:59:59Z").await;
        insert_news_truncation_audit(&storage, "kept-audit", "2026-05-04T00:00:00Z").await;

        let glossary_count_before = count_rows(&storage, "translation_glossary_entries").await;
        let migrations_count_before = count_rows(&storage, "schema_migrations").await;
        let now = OffsetDateTime::parse("2026-05-10T12:00:00Z", &Rfc3339)
            .expect("fixed timestamp should parse");
        let report = storage
            .prune_news_retention(now, 7)
            .await
            .expect("retention should prune append-only audits");

        assert_eq!(report.oldest_audit_retained_day, "2026-05-04");
        assert_eq!(report.data_audit_retention_days, 7);
        assert_eq!(report.deleted_news_blacklist_hits, 1);
        assert_eq!(report.deleted_news_truncation_audits, 1);
        assert!(!row_exists_by_signature(&storage, "news_blacklist_hits", "old-hit").await);
        assert!(row_exists_by_signature(&storage, "news_blacklist_hits", "kept-hit").await);
        assert!(!row_exists_by_signature(&storage, "news_truncation_audit", "old-audit").await);
        assert!(row_exists_by_signature(&storage, "news_truncation_audit", "kept-audit").await);
        assert_eq!(
            count_rows(&storage, "translation_glossary_entries").await,
            glossary_count_before
        );
        assert_eq!(
            count_rows(&storage, "schema_migrations").await,
            migrations_count_before
        );
        assert_eq!(count_rows(&storage, "news_blacklist_entries").await, 1);
        assert_eq!(
            count_rows(&storage, "daily_summary_scheduler_state").await,
            1
        );
    }

    async fn test_storage() -> SqliteNewsStore {
        let config = StorageConfig {
            database_url: "sqlite::memory:".to_owned(),
            max_connections: 1,
            data_audit_retention_days: 7,
        };
        let storage = SqliteNewsStore::connect(&config)
            .await
            .expect("in-memory sqlite should open");
        storage
            .migrate()
            .await
            .expect("schema migration should succeed");
        storage
    }

    async fn test_storage_with_calendar() -> SqliteNewsStore {
        let storage = test_storage().await;
        CalendarDayStore::from_news_store(&storage)
            .migrate()
            .await
            .expect("calendar schema migration should succeed");
        storage
    }

    async fn insert_translation_row(
        storage: &SqliteNewsStore,
        news_event_id: i64,
        language: &str,
        published_at: &str,
        created_at: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO news_translations (
                news_event_id,
                language,
                status,
                source,
                event_kind,
                source_title,
                translated_title,
                narration_text,
                translation_mode,
                published_at,
                created_at,
                updated_at
            ) VALUES (?1, ?2, 'ready', 'financial_juice', 'headline', 'Source title',
                'Titre traduit', '10:00. Titre traduit.', 'heuristic_fr', ?3, ?4, ?4)
            "#,
        )
        .bind(news_event_id)
        .bind(language)
        .bind(published_at)
        .bind(created_at)
        .execute(&storage.pool)
        .await
        .expect("translation fixture should insert");
    }

    async fn count_rows(storage: &SqliteNewsStore, table_name: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        sqlx::query_scalar::<_, i64>(&sql)
            .fetch_one(&storage.pool)
            .await
            .expect("row count should succeed")
    }

    async fn row_exists_by_run_id(
        storage: &SqliteNewsStore,
        table_name: &str,
        run_id: &str,
    ) -> bool {
        let sql = format!("SELECT COUNT(*) FROM {table_name} WHERE run_id = ?1");
        sqlx::query_scalar::<_, i64>(&sql)
            .bind(run_id)
            .fetch_one(&storage.pool)
            .await
            .expect("row count by run id should succeed")
            > 0
    }

    async fn row_exists_by_signature(
        storage: &SqliteNewsStore,
        table_name: &str,
        signature: &str,
    ) -> bool {
        let sql = format!("SELECT COUNT(*) FROM {table_name} WHERE signature = ?1");
        sqlx::query_scalar::<_, i64>(&sql)
            .bind(signature)
            .fetch_one(&storage.pool)
            .await
            .expect("row count by signature should succeed")
            > 0
    }

    async fn insert_daily_summary_run(storage: &SqliteNewsStore, run_id: &str, local_day: &str) {
        sqlx::query(
            r#"
            INSERT INTO daily_summary_runs (
                run_id,
                local_day,
                status,
                reason,
                forced,
                created_at,
                updated_at
            ) VALUES (?1, ?2, 'audio_ready', 'test', 0, ?3, ?3)
            "#,
        )
        .bind(run_id)
        .bind(local_day)
        .bind(format!("{local_day}T08:00:00Z"))
        .execute(&storage.pool)
        .await
        .expect("daily summary run fixture should insert");
    }

    async fn insert_daily_summary_run_step(
        storage: &SqliteNewsStore,
        run_id: &str,
        local_day: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO daily_summary_run_steps (
                run_id,
                local_day,
                stage,
                status,
                message,
                created_at
            ) VALUES (?1, ?2, 'fixture', 'ok', 'fixture step', ?3)
            "#,
        )
        .bind(run_id)
        .bind(local_day)
        .bind(format!("{local_day}T08:00:01Z"))
        .execute(&storage.pool)
        .await
        .expect("daily summary run step fixture should insert");
    }

    async fn insert_daily_summary_run_artifact(
        storage: &SqliteNewsStore,
        run_id: &str,
        local_day: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO daily_summary_run_artifacts (
                run_id,
                local_day,
                kind,
                label,
                content,
                created_at
            ) VALUES (?1, ?2, 'fixture', 'fixture artifact', '{}', ?3)
            "#,
        )
        .bind(run_id)
        .bind(local_day)
        .bind(format!("{local_day}T08:00:02Z"))
        .execute(&storage.pool)
        .await
        .expect("daily summary run artifact fixture should insert");
    }

    async fn insert_news_blacklist_entry(storage: &SqliteNewsStore, term: &str, created_at: &str) {
        sqlx::query(
            r#"
            INSERT INTO news_blacklist_entries (
                term,
                match_mode,
                target_scope,
                enabled,
                created_at,
                updated_at
            ) VALUES (?1, 'contains', 'title_content', 1, ?2, ?2)
            "#,
        )
        .bind(term)
        .bind(created_at)
        .execute(&storage.pool)
        .await
        .expect("blacklist entry fixture should insert");
    }

    async fn insert_daily_summary_scheduler_state(
        storage: &SqliteNewsStore,
        state_key: &str,
        updated_at: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO daily_summary_scheduler_state (
                state_key,
                last_news_event_id,
                updated_at
            ) VALUES (?1, 42, ?2)
            "#,
        )
        .bind(state_key)
        .bind(updated_at)
        .execute(&storage.pool)
        .await
        .expect("scheduler state fixture should insert");
    }

    async fn insert_news_blacklist_hit(
        storage: &SqliteNewsStore,
        signature: &str,
        created_at: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO news_blacklist_hits (
                term,
                match_mode,
                target_scope,
                action,
                source,
                event_kind,
                matched_field,
                matched_text,
                source_title,
                signature,
                created_at
            ) VALUES ('blocked', 'contains', 'title_content', 'reject_news',
                'financial_juice', 'headline', 'title', 'blocked text', 'Blocked title', ?1, ?2)
            "#,
        )
        .bind(signature)
        .bind(created_at)
        .execute(&storage.pool)
        .await
        .expect("blacklist hit fixture should insert");
    }

    async fn insert_news_truncation_audit(
        storage: &SqliteNewsStore,
        signature: &str,
        created_at: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO news_truncation_audit (
                source,
                field_name,
                marker,
                source_title,
                field_excerpt,
                signature,
                created_at
            ) VALUES ('financial_juice', 'body', '[truncated]',
                'Truncated title', 'Truncated body', ?1, ?2)
            "#,
        )
        .bind(signature)
        .bind(created_at)
        .execute(&storage.pool)
        .await
        .expect("truncation audit fixture should insert");
    }

    async fn insert_calendar_state(storage: &SqliteNewsStore, local_day: &str) {
        sqlx::query(
            r#"
            INSERT INTO calendar_day_state (
                local_day,
                calendar_ingestion_status,
                calendar_summary_status,
                created_at,
                updated_at
            ) VALUES (?1, 'ready', 'audio_ready', ?2, ?2)
            "#,
        )
        .bind(local_day)
        .bind(format!("{local_day}T08:00:00Z"))
        .execute(&storage.pool)
        .await
        .expect("calendar state fixture should insert");
    }

    async fn calendar_state_exists(storage: &SqliteNewsStore, local_day: &str) -> bool {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM calendar_day_state WHERE local_day = ?1")
            .bind(local_day)
            .fetch_one(&storage.pool)
            .await
            .expect("calendar state count should succeed")
            > 0
    }
}
