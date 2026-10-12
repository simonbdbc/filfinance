//! Creation idempotente des tables SQLite news.

use crate::errors::AppError;

/// Crée la table de suivi des migrations appliquées.
pub(super) async fn create_schema_migrations_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    // Cette table permet de tracer les migrations appliquées même si elles restent simples pour le MVP.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Crée la table principale des événements news.
pub(super) async fn create_news_events_table(pool: &sqlx::SqlitePool) -> Result<(), AppError> {
    // Chaque `CREATE TABLE IF NOT EXISTS` garde la migration idempotente au redémarrage.
    // Le schéma principal supporte déjà les headlines et les métadonnées macro.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS news_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            event_kind TEXT NOT NULL,
            title TEXT NOT NULL,
            url TEXT NOT NULL,
            published_at TEXT,
            event_time TEXT,
            ingested_at TEXT NOT NULL,
            body TEXT,
            news_level TEXT,
            importance TEXT,
            country_code TEXT,
            currency TEXT,
            actual_value TEXT,
            forecast_value TEXT,
            previous_value TEXT,
            raw_payload_ref TEXT,
            signature TEXT NOT NULL UNIQUE
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Crée la table des variantes texte dérivées.
pub(super) async fn create_news_translations_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    // Cette table porte les variantes texte dérivées sans alourdir le modèle canonique.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS news_translations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            news_event_id INTEGER NOT NULL,
            language TEXT NOT NULL,
            status TEXT NOT NULL,
            source TEXT NOT NULL,
            event_kind TEXT NOT NULL,
            source_title TEXT NOT NULL,
            source_body TEXT,
            translated_title TEXT NOT NULL,
            translated_body TEXT,
            narration_text TEXT NOT NULL,
            translation_mode TEXT NOT NULL,
            error_message TEXT,
            published_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(news_event_id, language)
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table des resumes IA journaliers.
pub(super) async fn create_daily_summaries_table(pool: &sqlx::SqlitePool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS daily_summaries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            local_day TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            summary_kind TEXT NOT NULL DEFAULT 'ai',
            status TEXT NOT NULL,
            title TEXT,
            daily_context_text TEXT,
            important_news_summary_text TEXT,
            summary_text TEXT,
            narration_text TEXT,
            stream_id TEXT,
            playlist_url TEXT,
            news_count INTEGER NOT NULL,
            important_news_count INTEGER NOT NULL DEFAULT 0,
            source_urls_json TEXT NOT NULL,
            error_message TEXT,
            provider TEXT,
            model TEXT,
            agent_latency_ms INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(local_day, content_hash)
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table des qualifications journalieres produites par n8n/OpenRouter.
pub(super) async fn create_daily_news_qualifications_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS daily_news_qualifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            local_day TEXT NOT NULL,
            news_event_id INTEGER NOT NULL,
            translation_id INTEGER,
            category TEXT NOT NULL,
            importance TEXT NOT NULL,
            summary TEXT NOT NULL,
            reasoning TEXT NOT NULL,
            confidence REAL,
            is_important INTEGER NOT NULL DEFAULT 0,
            provider TEXT,
            model TEXT,
            qualified_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(local_day, news_event_id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table de watermark du scheduler de resume journalier.
pub(super) async fn create_daily_summary_scheduler_state_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS daily_summary_scheduler_state (
            state_key TEXT PRIMARY KEY,
            last_news_event_id INTEGER NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table des runs admin du resume journalier.
pub(super) async fn create_daily_summary_runs_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS daily_summary_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id TEXT NOT NULL UNIQUE,
            local_day TEXT NOT NULL,
            summary_id INTEGER,
            status TEXT NOT NULL,
            reason TEXT NOT NULL,
            forced INTEGER NOT NULL DEFAULT 0,
            error_message TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table des etapes d'un run admin du resume journalier.
pub(super) async fn create_daily_summary_run_steps_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS daily_summary_run_steps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id TEXT NOT NULL,
            local_day TEXT NOT NULL,
            stage TEXT NOT NULL,
            status TEXT NOT NULL,
            message TEXT NOT NULL,
            detail TEXT,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table des artifacts redactes d'un run admin.
pub(super) async fn create_daily_summary_run_artifacts_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS daily_summary_run_artifacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id TEXT NOT NULL,
            local_day TEXT NOT NULL,
            kind TEXT NOT NULL,
            label TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table du glossaire metier traduction/TTS.
pub(super) async fn create_translation_glossary_entries_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS translation_glossary_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_term TEXT NOT NULL,
            translated_term TEXT NOT NULL,
            spoken_term TEXT,
            source_language TEXT NOT NULL,
            target_language TEXT NOT NULL,
            match_mode TEXT NOT NULL,
            case_sensitive INTEGER NOT NULL DEFAULT 0,
            priority INTEGER NOT NULL DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 1,
            scope TEXT NOT NULL DEFAULT 'full_pipeline',
            notes TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Ajoute les motifs glossaire de base sans dupliquer les entrees existantes.
pub(super) async fn seed_translation_glossary_pattern_entries(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    for seed in [
        TranslationGlossaryPatternSeed {
            source_term: "ECB's <string>",
            translated_term: "<string> de la BCE",
            spoken_term: None,
            priority: 100,
            scope: "full_pipeline",
            notes: "Motif directionnel: ECB's Nagel -> Nagel de la BCE.",
        },
        TranslationGlossaryPatternSeed {
            source_term: "<number>h00",
            translated_term: "<number>h00",
            spoken_term: Some("<number:hour>"),
            priority: 120,
            scope: "spoken_only",
            notes: "Diction horaire vocale: 14h00 -> quatorze heures.",
        },
        TranslationGlossaryPatternSeed {
            source_term: "<number>h<number>",
            translated_term: "<number>h<number>",
            spoken_term: Some("<number:hour> <number:minute>"),
            priority: 119,
            scope: "spoken_only",
            notes: "Diction horaire vocale: 12h01 -> douze heures une.",
        },
    ] {
        seed_translation_glossary_pattern_entry(pool, seed).await?;
    }

    Ok(())
}

struct TranslationGlossaryPatternSeed<'a> {
    source_term: &'a str,
    translated_term: &'a str,
    spoken_term: Option<&'a str>,
    priority: i64,
    scope: &'a str,
    notes: &'a str,
}

async fn seed_translation_glossary_pattern_entry(
    pool: &sqlx::SqlitePool,
    seed: TranslationGlossaryPatternSeed<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO translation_glossary_entries (
            source_term,
            translated_term,
            spoken_term,
            source_language,
            target_language,
            match_mode,
            case_sensitive,
            priority,
            enabled,
            scope,
            notes,
            created_at,
            updated_at
        )
        SELECT
            ?,
            ?,
            ?,
            'en',
            'fr',
            'pattern',
            0,
            ?,
            1,
            ?,
            ?,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
        WHERE NOT EXISTS (
            SELECT 1
            FROM translation_glossary_entries
            WHERE source_term = ?
              AND translated_term = ?
              AND match_mode = 'pattern'
              AND scope = ?
        );
        "#,
    )
    .bind(seed.source_term)
    .bind(seed.translated_term)
    .bind(seed.spoken_term)
    .bind(seed.priority)
    .bind(seed.scope)
    .bind(seed.notes)
    .bind(seed.source_term)
    .bind(seed.translated_term)
    .bind(seed.scope)
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table des regles blacklist news.
pub(super) async fn create_news_blacklist_entries_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS news_blacklist_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            term TEXT NOT NULL,
            match_mode TEXT NOT NULL,
            target_scope TEXT NOT NULL DEFAULT 'title_content',
            enabled INTEGER NOT NULL DEFAULT 1,
            notes TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table d'audit des news rejetees par blacklist.
pub(super) async fn create_news_blacklist_hits_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS news_blacklist_hits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            blacklist_entry_id INTEGER,
            term TEXT NOT NULL,
            match_mode TEXT NOT NULL,
            target_scope TEXT NOT NULL DEFAULT 'title_content',
            action TEXT NOT NULL DEFAULT 'reject_news',
            source TEXT NOT NULL,
            event_kind TEXT NOT NULL,
            matched_field TEXT NOT NULL,
            matched_text TEXT NOT NULL,
            source_title TEXT NOT NULL,
            source_body_excerpt TEXT,
            raw_payload_ref TEXT,
            signature TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cree la table d'audit des troncatures deja presentes upstream.
pub(super) async fn create_news_truncation_audit_table(
    pool: &sqlx::SqlitePool,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS news_truncation_audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            field_name TEXT NOT NULL,
            marker TEXT NOT NULL,
            source_title TEXT NOT NULL,
            field_excerpt TEXT NOT NULL,
            raw_payload_ref TEXT,
            signature TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(source, signature, field_name, marker)
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
