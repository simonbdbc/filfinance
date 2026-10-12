//! Calendrier du jour : ingestion native Rust, résumé, audio Piper et alertes.

// Les contrats du calendrier restent prives au service Rust.
// `cfg_attr` masque seulement les APIs de fixtures non appelees par le binaire,
// tandis que la compilation de test continue a signaler tout helper oublie.
#[cfg_attr(not(test), allow(dead_code))]
// `pub(crate)` limite la visibilite au service Rust et interdit d'en faire un contrat HTTP/public.
pub(crate) mod contracts;

// Le chemin canonique isole ses collecteurs et sa file durable du pipeline live.
pub(crate) mod native;

use std::collections::HashSet;
use std::time::Duration;

use axum::{
    Json,
    extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code},
    extract::{Query, State},
    response::Response,
};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, SqlitePool, Transaction};
use time::{
    Date, Duration as TimeDuration, Month, OffsetDateTime, Time, Weekday,
    format_description::well_known::Rfc3339,
};
use tokio::{
    sync::watch,
    task::JoinHandle,
    time::{self as tokio_time, MissedTickBehavior},
};
use tracing::{info, warn};
use url::Url;

use crate::{
    app_state::AppState,
    errors::AppError,
    features::{
        auth::{self, CurrentUser},
        live::{
            dto::EdgeCalendarAlertReadyPayload,
            edge::{edge_event_id_for_calendar_alert, publish_calendar_alert_ready},
        },
    },
    shared::{
        text::normalize_whitespace,
        time::{europe_paris_day_for_utc, europe_paris_offset_for_utc, now_rfc3339},
    },
};

const CALENDAR_DAY_WS_AUTH_TIMEOUT_SECS: u64 = 5;
const CALENDAR_DAY_WS_HEARTBEAT_SECS: u64 = 20;
const CALENDAR_DAY_WS_PROTOCOL_VERSION: u8 = 1;
const CALENDAR_DAY_PENDING_RUN_STALE_SECS: i64 = 45 * 60;
/// Valeur filaire historique de l'API admin hebdomadaire.
///
/// L'implementation est desormais durable, mais les clients existants ont une
/// union fermee sur cette valeur : le transport ne doit donc pas exposer un
/// nouveau libelle pendant la migration.
const CALENDAR_WEEK_RUN_PUBLIC_MODE: &str = "sequential_memory";

/// Store SQLite du calendrier journalier canonique.
#[derive(Clone)]
pub struct CalendarDayStore {
    pool: SqlitePool,
}

/// Resultat du commit unique de l'ingestion native.
///
/// Les valeurs ne sont exposees qu'apres le `COMMIT`; les broadcasts admin
/// peuvent donc etre emis sans jamais annoncer un item ou un travail aval qui
/// serait ensuite annule par SQLite.
#[derive(Clone, Debug)]
pub(crate) struct CalendarNativeIngestionCommit {
    pub(crate) state: CalendarDayState,
    pub(crate) ingestion_run: CalendarDayRun,
    pub(crate) summary_run: CalendarDayRun,
    pub(crate) items: Vec<CalendarDayItem>,
    /// Artifact redacte, deja durable au meme commit que les audits source.
    pub(crate) source_audit_artifact: CalendarDayRunArtifact,
}

/// Resultat du commit unique du resume natif et de son intention audio.
#[derive(Clone, Debug)]
pub(crate) struct CalendarNativeSummaryCommit {
    pub(crate) state: CalendarDayState,
    pub(crate) run: CalendarDayRun,
    pub(crate) summary: CalendarDaySummary,
}

/// Resultat atomique de l'admission ou de la reactivation d'un HLS de resume.
///
/// Le resume ne devient visible comme `audio_generating` qu'apres que cette
/// intention durable existe dans SQLite; un reveil memoire perdu reste donc
/// recuperable au prochain poll ou apres restart.
#[derive(Debug)]
pub(crate) struct CalendarNativeSummaryAudioRepair {
    pub(crate) state: CalendarDayState,
    pub(crate) summary: CalendarDaySummary,
    pub(crate) run: Option<CalendarDayRun>,
    pub(crate) admission: native::work::CalendarWorkAdmission,
}

/// Etat visible retourne apres un echec audio terminal committe avec le
/// dead-letter correspondant. Seul un resume a un etat calendrier global a
/// diffuser; les clips item/alerte restent exposes par leurs lignes propres.
#[derive(Debug)]
pub(crate) struct CalendarNativeAudioTerminalCommit {
    pub(crate) state: Option<CalendarDayState>,
    pub(crate) summary: Option<CalendarDaySummary>,
    pub(crate) run: Option<CalendarDayRun>,
}

/// Resultat de la persistence HLS committee avec l'acquittement de la lease.
#[derive(Debug)]
pub(crate) struct CalendarNativeAudioReadyCommit {
    pub(crate) state: Option<CalendarDayState>,
    pub(crate) summary: Option<CalendarDaySummary>,
    pub(crate) run: Option<CalendarDayRun>,
    pub(crate) alert: Option<CalendarDayAlert>,
}

/// Valeurs HLS deja ecrites a rendre visibles dans la meme transaction que le
/// `complete` du travail audio. Aucun champ ne transporte le contenu audio.
#[derive(Debug)]
pub(crate) enum CalendarNativeAudioReadyTarget {
    Item {
        item_id: i64,
        stream_id: String,
        playlist_url: String,
    },
    Alert {
        alert_id: i64,
        stream_id: String,
        playlist_url: String,
    },
    Summary {
        summary_id: i64,
        stream_id: String,
        playlist_url: String,
        playlist_path: String,
        immutable_playlist_url: String,
        current_playlist_url: String,
    },
}

/// Verdict transactionnel de l'admission d'une ingestion native.
///
/// L'admin et le scheduler ne doivent jamais observer un run `pending` sans
/// intention durable correspondante. Une course vers un run deja pending reste
/// idempotente et ne reinitialise aucun etat.
#[derive(Debug)]
pub(crate) enum CalendarNativeIngestionAdmission {
    Started {
        state: CalendarDayState,
        run: CalendarDayRun,
        admission: native::work::CalendarWorkAdmission,
    },
    Existing(CalendarDayRun),
}

/// Verdict transactionnel de l'admission d'un resume natif.
#[derive(Debug)]
pub(crate) enum CalendarNativeSummaryAdmission {
    Started {
        state: CalendarDayState,
        run: CalendarDayRun,
        admission: native::work::CalendarWorkAdmission,
    },
    Existing(CalendarDayRun),
}

impl CalendarDayStore {
    /// Reutilise le pool SQLite news pour garder une seule base locale.
    pub(crate) fn from_news_store(storage: &crate::features::news::NewsStore) -> Self {
        Self {
            pool: storage.clone_pool_for_feature_store(),
        }
    }

    /// Cree les tables canonique calendar_day, separees du legacy daily_summary.
    pub(crate) async fn migrate(&self) -> Result<(), AppError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_state (
                local_day TEXT PRIMARY KEY,
                calendar_ingestion_status TEXT NOT NULL DEFAULT 'missing',
                calendar_summary_status TEXT NOT NULL DEFAULT 'missing',
                calendar_ingested_at TEXT,
                calendar_content_hash TEXT,
                summary_content_hash TEXT,
                last_run_id TEXT,
                last_error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL UNIQUE,
                local_day TEXT NOT NULL,
                workflow_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                reason TEXT,
                attempt INTEGER NOT NULL DEFAULT 0,
                error_message TEXT,
                n8n_workflow_id TEXT,
                n8n_workflow_name TEXT,
                n8n_execution_id TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        self.add_calendar_day_run_n8n_columns().await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_run_steps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                local_day TEXT NOT NULL,
                stage TEXT NOT NULL,
                status TEXT NOT NULL,
                message TEXT NOT NULL,
                detail TEXT,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_run_artifacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                local_day TEXT NOT NULL,
                kind TEXT NOT NULL,
                label TEXT NOT NULL,
                content_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                local_day TEXT NOT NULL,
                event_time TEXT,
                source_title TEXT NOT NULL,
                translated_title TEXT NOT NULL,
                tts_text TEXT NOT NULL,
                country TEXT,
                currency TEXT,
                forecast_value TEXT,
                previous_value TEXT,
                actual_value TEXT,
                source_importance TEXT,
                ai_importance TEXT,
                final_importance TEXT NOT NULL,
                category TEXT NOT NULL,
                reasoning TEXT,
                confidence REAL,
                content_hash TEXT NOT NULL,
                sources_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(local_day, content_hash)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_item_sources (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                calendar_item_id INTEGER NOT NULL,
                source TEXT NOT NULL,
                external_id TEXT,
                source_url TEXT,
                raw_title TEXT,
                raw_json TEXT,
                created_at TEXT NOT NULL,
                UNIQUE(calendar_item_id, source, external_id, source_url)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_item_audio (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                calendar_item_id INTEGER NOT NULL,
                audio_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                stream_id TEXT,
                playlist_url TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(calendar_item_id, audio_kind)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_summaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                local_day TEXT NOT NULL,
                title TEXT NOT NULL,
                summary_text TEXT NOT NULL,
                narration_text TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                status TEXT NOT NULL,
                high_event_count INTEGER NOT NULL DEFAULT 0,
                provider TEXT,
                model TEXT,
                stream_id TEXT,
                playlist_url TEXT,
                playlist_path TEXT,
                immutable_playlist_url TEXT,
                current_playlist_url TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(local_day, content_hash)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        self.add_calendar_summary_hls_columns().await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                local_day TEXT NOT NULL,
                calendar_item_id INTEGER NOT NULL,
                alert_offset_secs INTEGER NOT NULL,
                scheduled_at TEXT NOT NULL,
                status TEXT NOT NULL,
                stream_id TEXT,
                playlist_url TEXT,
                published_at TEXT,
                expires_at TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(calendar_item_id, alert_offset_secs)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        // Les preuves source natives ne retiennent jamais l'URL ni le corps :
        // seulement le verdict ferme, le hash et la taille. Elles restent dans
        // le domaine calendrier et ne partagent aucune table avec l'outbox.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_day_source_audits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                local_day TEXT NOT NULL,
                source TEXT NOT NULL,
                outcome TEXT NOT NULL,
                detail TEXT NOT NULL,
                http_status INTEGER,
                raw_sha256 TEXT,
                raw_length_bytes INTEGER,
                created_at TEXT NOT NULL,
                UNIQUE(run_id, source)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        // La queue est appliquee avec les autres migrations calendrier avant
        // toute admission HTTP ou scheduler. Son schema reste volontairement
        // separe des files FinancialJuice/traduction.
        native::work::apply_schema(&self.pool).await?;
        for statement in [
            "CREATE INDEX IF NOT EXISTS idx_calendar_day_runs_local_day_created_at ON calendar_day_runs (local_day, created_at DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_calendar_day_run_steps_run_id_created_at ON calendar_day_run_steps (run_id, created_at ASC, id ASC)",
            "CREATE INDEX IF NOT EXISTS idx_calendar_day_run_artifacts_run_id_created_at ON calendar_day_run_artifacts (run_id, created_at ASC, id ASC)",
            "CREATE INDEX IF NOT EXISTS idx_calendar_day_items_local_day_time ON calendar_day_items (local_day, event_time ASC, id ASC)",
            "CREATE INDEX IF NOT EXISTS idx_calendar_day_items_local_day_importance ON calendar_day_items (local_day, final_importance, event_time ASC)",
            "CREATE INDEX IF NOT EXISTS idx_calendar_day_summaries_local_day_updated ON calendar_day_summaries (local_day, updated_at DESC, id DESC)",
            "CREATE INDEX IF NOT EXISTS idx_calendar_day_alerts_due ON calendar_day_alerts (status, scheduled_at ASC, id ASC)",
            "CREATE INDEX IF NOT EXISTS idx_calendar_day_source_audits_run ON calendar_day_source_audits (run_id, created_at ASC, id ASC)",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO schema_migrations (name)
            VALUES ('0028_add_calendar_native_work')
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn add_calendar_day_run_n8n_columns(&self) -> Result<(), AppError> {
        crate::features::calendar_logs::operation(
            "add_calendar_day_run_n8n_columns",
            serde_json::json!({}),
            async {
                for (column, definition) in [
                    ("n8n_workflow_id", "TEXT"),
                    ("n8n_workflow_name", "TEXT"),
                    ("n8n_execution_id", "TEXT"),
                ] {
                    if !self.calendar_day_runs_has_column(column).await? {
                        sqlx::query(&format!(
                            "ALTER TABLE calendar_day_runs ADD COLUMN {column} {definition}"
                        ))
                        .execute(&self.pool)
                        .await?;
                    }
                }
                sqlx::query(
                    r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                name TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
                )
                .execute(&self.pool)
                .await?;
                sqlx::query(
                    r#"
            INSERT OR IGNORE INTO schema_migrations (name)
            VALUES ('0019_add_calendar_day_n8n_metadata')
            "#,
                )
                .execute(&self.pool)
                .await?;
                Ok(())
            },
        )
        .await
    }

    async fn calendar_day_runs_has_column(&self, column_name: &str) -> Result<bool, AppError> {
        crate::features::calendar_logs::operation(
            "calendar_day_runs_has_column",
            serde_json::json!({"column_name": format!("{:?}", column_name)}),
            async {
                let count: i64 = sqlx::query_scalar(
                    r#"
            SELECT COUNT(*)
            FROM pragma_table_info('calendar_day_runs')
            WHERE name = ?1
            "#,
                )
                .bind(column_name)
                .fetch_one(&self.pool)
                .await?;
                Ok(count > 0)
            },
        )
        .await
    }

    async fn add_calendar_summary_hls_columns(&self) -> Result<(), AppError> {
        crate::features::calendar_logs::operation(
            "add_calendar_summary_hls_columns",
            serde_json::json!({}),
            async {
                for (column, definition) in [
                    ("playlist_path", "TEXT"),
                    ("immutable_playlist_url", "TEXT"),
                    ("current_playlist_url", "TEXT"),
                ] {
                    if !self.calendar_day_summaries_has_column(column).await? {
                        sqlx::query(&format!(
                            "ALTER TABLE calendar_day_summaries ADD COLUMN {column} {definition}"
                        ))
                        .execute(&self.pool)
                        .await?;
                    }
                }
                sqlx::query(
                    r#"
            INSERT OR IGNORE INTO schema_migrations (name)
            VALUES ('0021_add_calendar_summary_hls_manifest_paths')
            "#,
                )
                .execute(&self.pool)
                .await?;
                Ok(())
            },
        )
        .await
    }

    async fn calendar_day_summaries_has_column(&self, column_name: &str) -> Result<bool, AppError> {
        crate::features::calendar_logs::operation(
            "calendar_day_summaries_has_column",
            serde_json::json!({"column_name": format!("{:?}", column_name)}),
            async {
                let count: i64 = sqlx::query_scalar(
                    r#"
            SELECT COUNT(*)
            FROM pragma_table_info('calendar_day_summaries')
            WHERE name = ?1
            "#,
                )
                .bind(column_name)
                .fetch_one(&self.pool)
                .await?;
                Ok(count > 0)
            },
        )
        .await
    }

    async fn ensure_state(&self, local_day: &str) -> Result<CalendarDayState, AppError> {
        crate::features::calendar_logs::operation("ensure_state", serde_json::json!({"local_day": format!("{:?}", local_day)}), async {

        let now = now_rfc3339();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO calendar_day_state (
                local_day, calendar_ingestion_status, calendar_summary_status, created_at, updated_at
            )
            VALUES (?1, 'missing', 'missing', ?2, ?2)
            "#,
        )
        .bind(local_day)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        self.get_state(local_day).await?.ok_or_else(|| {
            AppError::storage(format!(
                "calendar day state missing after ensure: {local_day}"
            ))
        })

}).await
    }

    pub(crate) async fn get_state(
        &self,
        local_day: &str,
    ) -> Result<Option<CalendarDayState>, AppError> {
        crate::features::calendar_logs::operation(
            "get_state",
            serde_json::json!({"local_day": format!("{:?}", local_day)}),
            async {
                let item = sqlx::query_as::<_, CalendarDayState>(
                    r#"
            SELECT local_day, calendar_ingestion_status, calendar_summary_status,
                   calendar_ingested_at, calendar_content_hash, summary_content_hash,
                   last_run_id, last_error, created_at, updated_at
            FROM calendar_day_state
            WHERE local_day = ?1
            "#,
                )
                .bind(local_day)
                .fetch_optional(&self.pool)
                .await?;
                Ok(item)
            },
        )
        .await
    }

    async fn update_state_status(
        &self,
        local_day: &str,
        ingestion_status: Option<&str>,
        summary_status: Option<&str>,
        last_run_id: Option<&str>,
        last_error: Option<&str>,
    ) -> Result<CalendarDayState, AppError> {
        crate::features::calendar_logs::operation("update_state_status", serde_json::json!({"local_day": format!("{:?}", local_day), "ingestion_status": format!("{:?}", ingestion_status), "summary_status": format!("{:?}", summary_status), "last_run_id": format!("{:?}", last_run_id), "last_error": format!("{:?}", last_error)}), async {

        let _ = self.ensure_state(local_day).await?;
        let now = now_rfc3339();
        if let Some(status) = ingestion_status {
            sqlx::query(
                r#"
                UPDATE calendar_day_state
                SET calendar_ingestion_status = ?2,
                    last_run_id = COALESCE(?3, last_run_id),
                    last_error = ?4,
                    updated_at = ?5
                WHERE local_day = ?1
                "#,
            )
            .bind(local_day)
            .bind(status)
            .bind(last_run_id)
            .bind(last_error)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }
        if let Some(status) = summary_status {
            sqlx::query(
                r#"
                UPDATE calendar_day_state
                SET calendar_summary_status = ?2,
                    last_run_id = COALESCE(?3, last_run_id),
                    last_error = ?4,
                    updated_at = ?5
                WHERE local_day = ?1
                "#,
            )
            .bind(local_day)
            .bind(status)
            .bind(last_run_id)
            .bind(last_error)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }
        Ok(self
            .get_state(local_day)
            .await?
            .expect("calendar day state exists"))

}).await
    }

    /// Transforme une anomalie de manifeste en intention `SummaryAudio`
    /// durable, sans passage direct par le canal Piper/HLS.
    ///
    /// Le changement de statut metier et l'admission/re-activation de la file
    /// partagent la meme transaction. Ainsi, une saturation de la file laisse
    /// le resume `audio_ready`, et un crash apres commit est repris depuis
    /// SQLite avec le meme stream reserve.
    pub(crate) async fn admit_native_summary_audio_repair(
        &self,
        work: &native::work::CalendarWorkStore,
        summary_id: i64,
        now: OffsetDateTime,
    ) -> Result<Option<CalendarNativeSummaryAudioRepair>, AppError> {
        crate::features::calendar_logs::operation("admit_native_summary_audio_repair", serde_json::json!({"summary_id": format!("{:?}", summary_id), "now": format!("{:?}", now)}), async {

        let now_text = native_calendar_timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        let Some(summary) =
            sqlx::query_as::<_, CalendarDaySummary>(&calendar_day_summary_select("WHERE id = ?1"))
                .bind(summary_id)
                .fetch_optional(&mut *tx)
                .await?
        else {
            return Ok(None);
        };
        let was_audio_ready = summary.status == "audio_ready";
        if !was_audio_ready && summary.status != "audio_generating" {
            return Ok(None);
        }

        let request = native::work::CalendarWorkRequest::try_new(
            native::work::CalendarWorkKind::SummaryAudio,
            &summary.local_day,
            format!("summary-{}", summary.id),
        )?;
        let admission = work
            .reactivate_audio_for_repair_in_tx(&mut tx, &request, summary.stream_id.as_deref(), now)
            .await?;

        if was_audio_ready {
            let updated = sqlx::query(
                r#"
                UPDATE calendar_day_summaries
                SET status = 'audio_generating',
                    playlist_url = NULL,
                    playlist_path = NULL,
                    immutable_playlist_url = NULL,
                    current_playlist_url = NULL,
                    error_message = NULL,
                    updated_at = ?2
                WHERE id = ?1 AND status = 'audio_ready'
                "#,
            )
            .bind(summary.id)
            .bind(&now_text)
            .execute(&mut *tx)
            .await?;
            if updated.rows_affected() != 1 {
                // Une autre requete GET publique a deja admis la meme
                // reparation entre la lecture et cette ecriture. Ne pas
                // transformer cette course normale en 5xx : le rollback
                // laisse l'intention/lease du premier caller intacte et le
                // handler relira l'etat courant avant de repondre.
                return Ok(None);
            }
        }
        native_ensure_state_tx(&mut tx, &summary.local_day, &now_text).await?;
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_summary_status = 'audio_generating',
                last_error = NULL,
                updated_at = ?2
            WHERE local_day = ?1
            "#,
        )
        .bind(&summary.local_day)
        .bind(&now_text)
        .execute(&mut *tx)
        .await?;
        let repair_run = sqlx::query_as::<_, CalendarDayRun>(
            r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE local_day = ?1 AND workflow_kind = 'summary'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(&summary.local_day)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(run) = repair_run.as_ref() {
            sqlx::query(
                r#"
                UPDATE calendar_day_runs
                SET status = 'audio_generating',
                    error_message = NULL,
                    completed_at = NULL,
                    updated_at = ?2
                WHERE run_id = ?1
                "#,
            )
            .bind(&run.run_id)
            .bind(&now_text)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        let state = self.get_state(&summary.local_day).await?.ok_or_else(|| {
            AppError::storage("calendar_native_state_missing_after_summary_audio_repair")
        })?;
        let summary = self.get_summary_by_id(summary.id).await?.ok_or_else(|| {
            AppError::storage("calendar_native_summary_missing_after_audio_repair")
        })?;
        let run = match repair_run {
            Some(run) => self.get_run(&run.run_id).await?,
            None => None,
        };
        Ok(Some(CalendarNativeSummaryAudioRepair {
            state,
            summary,
            run,
            admission,
        }))

}).await
    }

    /// Rend un clip HLS visible et acquitte son travail SQLite sous le meme
    /// fencing token. Si la lease a expire entre Piper/HLS et ce commit,
    /// aucune ligne metier n'est modifiee par l'ancien worker.
    pub(crate) async fn commit_native_audio_ready(
        &self,
        work: &native::work::CalendarWorkStore,
        lease: &native::work::CalendarWorkLease,
        target: CalendarNativeAudioReadyTarget,
        now: OffsetDateTime,
    ) -> Result<CalendarNativeAudioReadyCommit, AppError> {
        crate::features::calendar_logs::operation("commit_native_audio_ready", serde_json::json!({"lease": format!("{:?}", lease), "target": format!("{:?}", target), "now": format!("{:?}", now)}), async {

        let now_text = native_calendar_timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        if !work.lease_is_current_in_tx(&mut tx, lease, now).await? {
            return Err(AppError::conflict("calendar_native_work_fenced_out"));
        }

        let mut state = None;
        let mut summary = None;
        let mut run_id = None;
        let mut alert = None;
        match target {
            CalendarNativeAudioReadyTarget::Item {
                item_id,
                stream_id,
                playlist_url,
            } => {
                let updated = sqlx::query(
                    r#"
                    UPDATE calendar_day_item_audio
                    SET status = 'audio_ready', stream_id = ?2, playlist_url = ?3,
                        error_message = NULL, updated_at = ?4
                    WHERE calendar_item_id = ?1 AND audio_kind = 'event_high'
                    "#,
                )
                .bind(item_id)
                .bind(&stream_id)
                .bind(&playlist_url)
                .bind(&now_text)
                .execute(&mut *tx)
                .await?;
                if updated.rows_affected() != 1 {
                    return Err(AppError::storage(
                        "calendar_native_item_audio_ready_target_missing",
                    ));
                }
            }
            CalendarNativeAudioReadyTarget::Alert {
                alert_id,
                stream_id,
                playlist_url,
            } => {
                let updated = sqlx::query(
                    r#"
                    UPDATE calendar_day_alerts
                    SET status = 'scheduled', stream_id = ?2, playlist_url = ?3,
                        error_message = NULL, updated_at = ?4
                    WHERE id = ?1
                    "#,
                )
                .bind(alert_id)
                .bind(&stream_id)
                .bind(&playlist_url)
                .bind(&now_text)
                .execute(&mut *tx)
                .await?;
                if updated.rows_affected() != 1 {
                    return Err(AppError::storage(
                        "calendar_native_alert_audio_ready_target_missing",
                    ));
                }
                alert = Some(alert_id);
            }
            CalendarNativeAudioReadyTarget::Summary {
                summary_id,
                stream_id,
                playlist_url,
                playlist_path,
                immutable_playlist_url,
                current_playlist_url,
            } => {
                let Some(existing) = sqlx::query_as::<_, CalendarDaySummary>(
                    &calendar_day_summary_select("WHERE id = ?1"),
                )
                .bind(summary_id)
                .fetch_optional(&mut *tx)
                .await?
                else {
                    return Err(AppError::storage(
                        "calendar_native_summary_audio_ready_target_missing",
                    ));
                };
                let updated = sqlx::query(
                    r#"
                    UPDATE calendar_day_summaries
                    SET status = 'audio_ready',
                        stream_id = ?2,
                        playlist_url = ?3,
                        playlist_path = ?4,
                        immutable_playlist_url = ?5,
                        current_playlist_url = ?6,
                        error_message = NULL,
                        updated_at = ?7
                    WHERE id = ?1
                    "#,
                )
                .bind(summary_id)
                .bind(&stream_id)
                .bind(&playlist_url)
                .bind(&playlist_path)
                .bind(&immutable_playlist_url)
                .bind(&current_playlist_url)
                .bind(&now_text)
                .execute(&mut *tx)
                .await?;
                if updated.rows_affected() != 1 {
                    return Err(AppError::storage(
                        "calendar_native_summary_audio_ready_update_missing",
                    ));
                }
                native_ensure_state_tx(&mut tx, &existing.local_day, &now_text).await?;
                sqlx::query(
                    r#"
                    UPDATE calendar_day_state
                    SET calendar_summary_status = 'audio_ready',
                        summary_content_hash = ?2,
                        last_error = NULL,
                        updated_at = ?3
                    WHERE local_day = ?1
                    "#,
                )
                .bind(&existing.local_day)
                .bind(&existing.content_hash)
                .bind(&now_text)
                .execute(&mut *tx)
                .await?;
                state = Some(existing.local_day.clone());
                let run = sqlx::query_as::<_, CalendarDayRun>(
                    r#"
                    SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                           error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                           started_at, completed_at, created_at, updated_at
                    FROM calendar_day_runs
                    WHERE local_day = ?1 AND workflow_kind = 'summary'
                    ORDER BY created_at DESC, id DESC
                    LIMIT 1
                    "#,
                )
                .bind(&existing.local_day)
                .fetch_optional(&mut *tx)
                .await?;
                if let Some(run) = run {
                    sqlx::query(
                        r#"
                        UPDATE calendar_day_runs
                        SET status = 'audio_ready',
                            error_message = NULL,
                            completed_at = COALESCE(completed_at, ?2),
                            updated_at = ?2
                        WHERE run_id = ?1
                        "#,
                    )
                    .bind(&run.run_id)
                    .bind(&now_text)
                    .execute(&mut *tx)
                    .await?;
                    run_id = Some(run.run_id);
                }
                summary = Some(summary_id);
            }
        }

        match work.complete_in_tx(&mut tx, lease, now).await? {
            native::work::CalendarWorkLeaseMutation::Applied(_) => {}
            native::work::CalendarWorkLeaseMutation::FencedOut => {
                return Err(AppError::conflict("calendar_native_work_fenced_out"));
            }
        }
        tx.commit().await?;

        let state = match state {
            Some(local_day) => Some(self.get_state(&local_day).await?.ok_or_else(|| {
                AppError::storage("calendar_native_state_missing_after_audio_ready")
            })?),
            None => None,
        };
        let summary = match summary {
            Some(summary_id) => self.get_summary_by_id(summary_id).await?,
            None => None,
        };
        let run = match run_id {
            Some(run_id) => self.get_run(&run_id).await?,
            None => None,
        };
        let alert = match alert {
            Some(alert_id) => self.get_alert_by_id(alert_id).await?,
            None => None,
        };
        Ok(CalendarNativeAudioReadyCommit {
            state,
            summary,
            run,
            alert,
        })

}).await
    }

    /// Commit atomique d'un dead-letter audio et de l'etat calendrier cible.
    ///
    /// La file et la ligne visible basculent ensemble sous le meme fencing
    /// token. Un `SummaryAudio` ne peut donc jamais etre definitivement perdu
    /// tout en laissant le resume en `audio_generating`.
    pub(crate) async fn commit_native_audio_terminal_error(
        &self,
        work: &native::work::CalendarWorkStore,
        lease: &native::work::CalendarWorkLease,
        kind: native::work::CalendarWorkKind,
        target_id: i64,
        error_code: &native::work::CalendarWorkErrorCode,
        now: OffsetDateTime,
    ) -> Result<CalendarNativeAudioTerminalCommit, AppError> {
        crate::features::calendar_logs::operation("commit_native_audio_terminal_error", serde_json::json!({"lease": format!("{:?}", lease), "kind": format!("{:?}", kind), "target_id": format!("{:?}", target_id), "error_code": format!("{:?}", error_code), "now": format!("{:?}", now)}), async {

        if target_id <= 0 {
            return Err(AppError::storage(
                "calendar_native_audio_terminal_target_invalid",
            ));
        }
        let now_text = native_calendar_timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        if !work.lease_is_current_in_tx(&mut tx, lease, now).await? {
            return Err(AppError::conflict("calendar_native_work_fenced_out"));
        }

        let mut state = None;
        let mut summary = None;
        let mut terminal_run_id = None;
        match kind {
            native::work::CalendarWorkKind::ItemAudio => {
                let updated = sqlx::query(
                    r#"
                    UPDATE calendar_day_item_audio
                    SET status = 'error', error_message = ?2, updated_at = ?3
                    WHERE calendar_item_id = ?1 AND audio_kind = 'event_high'
                    "#,
                )
                .bind(target_id)
                .bind(error_code.as_public_code())
                .bind(&now_text)
                .execute(&mut *tx)
                .await?;
                if updated.rows_affected() != 1 {
                    return Err(AppError::storage(
                        "calendar_native_item_audio_terminal_target_missing",
                    ));
                }
            }
            native::work::CalendarWorkKind::AlertAudio => {
                let updated = sqlx::query(
                    r#"
                    UPDATE calendar_day_alerts
                    SET status = 'error', error_message = ?2, updated_at = ?3
                    WHERE id = ?1
                    "#,
                )
                .bind(target_id)
                .bind(error_code.as_public_code())
                .bind(&now_text)
                .execute(&mut *tx)
                .await?;
                if updated.rows_affected() != 1 {
                    return Err(AppError::storage(
                        "calendar_native_alert_audio_terminal_target_missing",
                    ));
                }
            }
            native::work::CalendarWorkKind::SummaryAudio => {
                let Some(existing) = sqlx::query_as::<_, CalendarDaySummary>(
                    &calendar_day_summary_select("WHERE id = ?1"),
                )
                .bind(target_id)
                .fetch_optional(&mut *tx)
                .await?
                else {
                    return Err(AppError::storage(
                        "calendar_native_summary_audio_terminal_target_missing",
                    ));
                };
                let updated = sqlx::query(
                    r#"
                    UPDATE calendar_day_summaries
                    SET status = 'error',
                        playlist_url = NULL,
                        playlist_path = NULL,
                        immutable_playlist_url = NULL,
                        current_playlist_url = NULL,
                        error_message = ?2,
                        updated_at = ?3
                    WHERE id = ?1
                    "#,
                )
                .bind(target_id)
                .bind(error_code.as_public_code())
                .bind(&now_text)
                .execute(&mut *tx)
                .await?;
                if updated.rows_affected() != 1 {
                    return Err(AppError::storage(
                        "calendar_native_summary_audio_terminal_update_missing",
                    ));
                }
                native_ensure_state_tx(&mut tx, &existing.local_day, &now_text).await?;
                sqlx::query(
                    r#"
                    UPDATE calendar_day_state
                    SET calendar_summary_status = 'error',
                        last_error = ?2,
                        updated_at = ?3
                    WHERE local_day = ?1
                    "#,
                )
                .bind(&existing.local_day)
                .bind(error_code.as_public_code())
                .bind(&now_text)
                .execute(&mut *tx)
                .await?;
                state = Some(existing.local_day.clone());
                let run = sqlx::query_as::<_, CalendarDayRun>(
                    r#"
                    SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                           error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                           started_at, completed_at, created_at, updated_at
                    FROM calendar_day_runs
                    WHERE local_day = ?1 AND workflow_kind = 'summary'
                    ORDER BY created_at DESC, id DESC
                    LIMIT 1
                    "#,
                )
                .bind(&existing.local_day)
                .fetch_optional(&mut *tx)
                .await?;
                if let Some(run) = run {
                    sqlx::query(
                        r#"
                        UPDATE calendar_day_runs
                        SET status = 'error',
                            error_message = ?2,
                            completed_at = COALESCE(completed_at, ?3),
                            updated_at = ?3
                        WHERE run_id = ?1
                        "#,
                    )
                    .bind(&run.run_id)
                    .bind(error_code.as_public_code())
                    .bind(&now_text)
                    .execute(&mut *tx)
                    .await?;
                    terminal_run_id = Some(run.run_id);
                }
            }
            native::work::CalendarWorkKind::Ingestion | native::work::CalendarWorkKind::Summary => {
                return Err(AppError::storage(
                    "calendar_native_audio_terminal_kind_invalid",
                ));
            }
        }

        match work.fail_in_tx(&mut tx, lease, error_code, now).await? {
            native::work::CalendarWorkLeaseMutation::Applied(_) => {}
            native::work::CalendarWorkLeaseMutation::FencedOut => {
                return Err(AppError::conflict("calendar_native_work_fenced_out"));
            }
        }
        tx.commit().await?;

        let state = match state {
            Some(local_day) => Some(self.get_state(&local_day).await?.ok_or_else(|| {
                AppError::storage("calendar_native_state_missing_after_audio_terminal")
            })?),
            None => None,
        };
        if matches!(kind, native::work::CalendarWorkKind::SummaryAudio) {
            summary = self.get_summary_by_id(target_id).await?;
        }
        let run = match terminal_run_id {
            Some(run_id) => self.get_run(&run_id).await?,
            None => None,
        };
        Ok(CalendarNativeAudioTerminalCommit {
            state,
            summary,
            run,
        })

}).await
    }

    async fn mark_summary_error(&self, summary_id: i64, error: &str) -> Result<(), AppError> {
        crate::features::calendar_logs::operation("mark_summary_error", serde_json::json!({"summary_id": format!("{:?}", summary_id), "error": format!("{:?}", error)}), async {

        let now = now_rfc3339();
        sqlx::query(
            r#"
            UPDATE calendar_day_summaries
            SET status = 'error',
                playlist_url = NULL,
                playlist_path = NULL,
                immutable_playlist_url = NULL,
                current_playlist_url = NULL,
                error_message = ?2,
                updated_at = ?3
            WHERE id = ?1
            "#,
        )
        .bind(summary_id)
        .bind(error)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        if let Some(summary) = self.get_summary_by_id(summary_id).await? {
            let _ = self
                .update_state_status(&summary.local_day, None, Some("error"), None, Some(error))
                .await?;
        }
        Ok(())

}).await
    }

    pub(crate) async fn insert_run(
        &self,
        local_day: &str,
        workflow_kind: &str,
        reason: &str,
        attempt: u32,
    ) -> Result<CalendarDayRun, AppError> {
        crate::features::calendar_logs::operation("insert_run", serde_json::json!({"local_day": format!("{:?}", local_day), "workflow_kind": format!("{:?}", workflow_kind), "reason": format!("{:?}", reason), "attempt": format!("{:?}", attempt)}), async {

        let now = now_rfc3339();
        let run_id = format!(
            "calendar-day-{workflow_kind}-{local_day}-{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        );
        sqlx::query(
            r#"
            INSERT INTO calendar_day_runs (
                run_id, local_day, workflow_kind, status, reason, attempt,
                started_at, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, ?6, ?6)
            "#,
        )
        .bind(&run_id)
        .bind(local_day)
        .bind(workflow_kind)
        .bind(reason)
        .bind(i64::from(attempt))
        .bind(&now)
        .execute(&self.pool)
        .await?;
        self.get_run(&run_id).await?.ok_or_else(|| {
            AppError::storage(format!("calendar day run missing after insert: {run_id}"))
        })

}).await
    }

    async fn update_run_status(
        &self,
        run_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<Option<CalendarDayRun>, AppError> {
        crate::features::calendar_logs::operation("update_run_status", serde_json::json!({"run_id": format!("{:?}", run_id), "status": format!("{:?}", status), "error_message": format!("{:?}", error_message)}), async {

        let now = now_rfc3339();
        let completed_at =
            matches!(status, "ready" | "audio_ready" | "error").then_some(now.as_str());
        sqlx::query(
            r#"
            UPDATE calendar_day_runs
            SET status = ?2, error_message = ?3, completed_at = COALESCE(?4, completed_at), updated_at = ?5
            WHERE run_id = ?1
            "#,
        )
        .bind(run_id)
        .bind(status)
        .bind(error_message)
        .bind(completed_at)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        self.get_run(run_id).await

}).await
    }

    async fn get_run(&self, run_id: &str) -> Result<Option<CalendarDayRun>, AppError> {
        crate::features::calendar_logs::operation(
            "get_run",
            serde_json::json!({"run_id": format!("{:?}", run_id)}),
            async {
                let item = sqlx::query_as::<_, CalendarDayRun>(
                    r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE run_id = ?1
            "#,
                )
                .bind(run_id)
                .fetch_optional(&self.pool)
                .await?;
                Ok(item)
            },
        )
        .await
    }

    async fn latest_run(&self, local_day: &str) -> Result<Option<CalendarDayRun>, AppError> {
        crate::features::calendar_logs::operation(
            "latest_run",
            serde_json::json!({"local_day": format!("{:?}", local_day)}),
            async {
                let item = sqlx::query_as::<_, CalendarDayRun>(
                    r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE local_day = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
                )
                .bind(local_day)
                .fetch_optional(&self.pool)
                .await?;
                Ok(item)
            },
        )
        .await
    }

    async fn list_runs(
        &self,
        local_day: &str,
        limit: i64,
    ) -> Result<Vec<CalendarDayRun>, AppError> {
        crate::features::calendar_logs::operation("list_runs", serde_json::json!({"local_day": format!("{:?}", local_day), "limit": format!("{:?}", limit)}), async {

        let items = sqlx::query_as::<_, CalendarDayRun>(
            r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE local_day = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT ?2
            "#,
        )
        .bind(local_day)
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;
        Ok(items)

}).await
    }

    async fn latest_run_by_kind(
        &self,
        local_day: &str,
        workflow_kind: &str,
    ) -> Result<Option<CalendarDayRun>, AppError> {
        crate::features::calendar_logs::operation("latest_run_by_kind", serde_json::json!({"local_day": format!("{:?}", local_day), "workflow_kind": format!("{:?}", workflow_kind)}), async {

        let item = sqlx::query_as::<_, CalendarDayRun>(
            r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE local_day = ?1 AND workflow_kind = ?2
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(local_day)
        .bind(workflow_kind)
        .fetch_optional(&self.pool)
        .await?;
        Ok(item)

}).await
    }

    async fn pending_run_by_kind(
        &self,
        local_day: &str,
        workflow_kind: &str,
    ) -> Result<Option<CalendarDayRun>, AppError> {
        crate::features::calendar_logs::operation("pending_run_by_kind", serde_json::json!({"local_day": format!("{:?}", local_day), "workflow_kind": format!("{:?}", workflow_kind)}), async {

        let item = sqlx::query_as::<_, CalendarDayRun>(
            r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE local_day = ?1 AND workflow_kind = ?2 AND status = 'pending'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(local_day)
        .bind(workflow_kind)
        .fetch_optional(&self.pool)
        .await?;
        Ok(item)

}).await
    }

    async fn insert_step(
        &self,
        run_id: &str,
        local_day: &str,
        stage: &str,
        status: &str,
        message: &str,
        detail: Option<&str>,
    ) -> Result<CalendarDayRunStep, AppError> {
        crate::features::calendar_logs::operation("insert_step", serde_json::json!({"run_id": format!("{:?}", run_id), "local_day": format!("{:?}", local_day), "stage": format!("{:?}", stage), "status": format!("{:?}", status), "message": format!("{:?}", message), "detail": format!("{:?}", detail)}), async {

        let now = now_rfc3339();
        let id = sqlx::query(
            r#"
            INSERT INTO calendar_day_run_steps (
                run_id, local_day, stage, status, message, detail, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
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
        .await?
        .last_insert_rowid();
        let item = sqlx::query_as::<_, CalendarDayRunStep>(
            r#"
            SELECT id, run_id, local_day, stage, status, message, detail, created_at
            FROM calendar_day_run_steps
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(item)

}).await
    }

    async fn list_steps(&self, run_id: &str) -> Result<Vec<CalendarDayRunStep>, AppError> {
        crate::features::calendar_logs::operation(
            "list_steps",
            serde_json::json!({"run_id": format!("{:?}", run_id)}),
            async {
                let items = sqlx::query_as::<_, CalendarDayRunStep>(
                    r#"
            SELECT id, run_id, local_day, stage, status, message, detail, created_at
            FROM calendar_day_run_steps
            WHERE run_id = ?1
            ORDER BY created_at ASC, id ASC
            "#,
                )
                .bind(run_id)
                .fetch_all(&self.pool)
                .await?;
                Ok(items)
            },
        )
        .await
    }

    async fn list_artifacts(&self, run_id: &str) -> Result<Vec<CalendarDayRunArtifact>, AppError> {
        crate::features::calendar_logs::operation(
            "list_artifacts",
            serde_json::json!({"run_id": format!("{:?}", run_id)}),
            async {
                let items = sqlx::query_as::<_, CalendarDayRunArtifact>(
                    r#"
            SELECT id, run_id, local_day, kind, label, content_json, created_at
            FROM calendar_day_run_artifacts
            WHERE run_id = ?1
            ORDER BY created_at ASC, id ASC
            "#,
                )
                .bind(run_id)
                .fetch_all(&self.pool)
                .await?;
                Ok(items)
            },
        )
        .await
    }

    /// Relit le dernier audit de collecte natif du jour, independamment du
    /// dernier run. L'ingestion cree ensuite un run de resume, qui deviendrait
    /// sinon le `latest_run` et masquerait cet artifact dans le snapshot admin.
    pub(crate) async fn latest_native_source_audit_artifact(
        &self,
        local_day: &str,
    ) -> Result<Option<CalendarDayRunArtifact>, AppError> {
        crate::features::calendar_logs::operation(
            "latest_native_source_audit_artifact",
            serde_json::json!({"local_day": format!("{:?}", local_day)}),
            async {
                let item = sqlx::query_as::<_, CalendarDayRunArtifact>(
                    r#"
            SELECT id, run_id, local_day, kind, label, content_json, created_at
            FROM calendar_day_run_artifacts
            WHERE local_day = ?1
              AND label = 'calendar_native_source_audits'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
                )
                .bind(local_day)
                .fetch_optional(&self.pool)
                .await?;
                Ok(item)
            },
        )
        .await
    }

    /// Relit les refus terminaux deja observes pour ce jour sans exposer le
    /// contenu source. Un run force peut encore collecter l'autre source
    /// autorisee, mais il ne doit jamais recontacter une source anti-bot/refusee.
    pub(crate) async fn native_collector_quarantines(
        &self,
        local_day: &str,
    ) -> Result<Vec<native::collectors::CalendarCollectorQuarantine>, AppError> {
        crate::features::calendar_logs::operation(
            "native_collector_quarantines",
            serde_json::json!({"local_day": format!("{:?}", local_day)}),
            async {
                #[derive(sqlx::FromRow)]
                struct RefusedSourceAudit {
                    source: String,
                    detail: String,
                    http_status: Option<i64>,
                }

                let rows = sqlx::query_as::<_, RefusedSourceAudit>(
                    r#"
            SELECT source, detail, http_status
            FROM calendar_day_source_audits
            WHERE local_day = ?1 AND outcome = 'refused'
            ORDER BY created_at DESC, id DESC
            "#,
                )
                .bind(local_day)
                .fetch_all(&self.pool)
                .await?;

                let mut quarantines: Vec<native::collectors::CalendarCollectorQuarantine> =
                    Vec::with_capacity(rows.len());
                for row in rows {
                    let Some(source) =
                        native::collectors::CalendarCollectorSource::from_name(&row.source)
                    else {
                        continue;
                    };
                    if quarantines.iter().any(
                        |quarantine: &native::collectors::CalendarCollectorQuarantine| {
                            quarantine.source == source
                        },
                    ) {
                        continue;
                    }
                    let reason = match row.detail.as_str() {
                        "unauthorized" => {
                            native::collectors::CalendarCollectorRefusalReason::Unauthorized
                        }
                        "forbidden" => {
                            native::collectors::CalendarCollectorRefusalReason::Forbidden
                        }
                        "rate_limited" => {
                            native::collectors::CalendarCollectorRefusalReason::RateLimited
                        }
                        "legal_restriction" => {
                            native::collectors::CalendarCollectorRefusalReason::LegalRestriction
                        }
                        // Une ligne historique malformee est conservee sous le refus
                        // le plus prudent: aucune requete ne doit contourner une
                        // protection deja signalee pour ce jour.
                        _ => native::collectors::CalendarCollectorRefusalReason::Challenge,
                    };
                    quarantines.push(native::collectors::CalendarCollectorQuarantine {
                        source,
                        reason,
                        http_status: row
                            .http_status
                            .and_then(|status| u16::try_from(status).ok()),
                    });
                }
                Ok(quarantines)
            },
        )
        .await
    }

    /// Rend les audits de collecte durables avant toute etape qui peut etre
    /// rejouee (notamment OpenRouter). Ainsi, un `403` ou challenge observe
    /// pendant un run dont l'IA doit etre retry ne provoque jamais une seconde
    /// requete vers la meme source le meme jour.
    ///
    /// L'artifact reutilise le contrat admin/WebSocket existant et ne contient
    /// que les metadonnees redigees de l'audit, jamais le snapshot collecte.
    pub(crate) async fn persist_native_source_audits(
        &self,
        work: &native::work::CalendarWorkStore,
        lease: &native::work::CalendarWorkLease,
        run_id: &str,
        local_day: &str,
        audits: &[native::collectors::CalendarCollectorAudit],
        now: OffsetDateTime,
    ) -> Result<CalendarDayRunArtifact, AppError> {
        crate::features::calendar_logs::operation("persist_native_source_audits", serde_json::json!({"lease": format!("{:?}", lease), "run_id": format!("{:?}", run_id), "local_day": format!("{:?}", local_day), "audits": format!("{:?}", audits), "now": format!("{:?}", now)}), async {

        let now_text = native_calendar_timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        if !work.lease_is_current_in_tx(&mut tx, lease, now).await? {
            return Err(AppError::conflict("calendar_native_work_fenced_out"));
        }
        native_ensure_state_tx(&mut tx, local_day, &now_text).await?;
        native_upsert_source_audits_tx(&mut tx, run_id, local_day, audits, &now_text).await?;
        let artifact =
            native_insert_source_audit_artifact_tx(&mut tx, run_id, local_day, audits, &now_text)
                .await?;
        tx.commit().await?;
        Ok(artifact)

}).await
    }

    /// Publie l'etat terminal redige apres le verrou fail-closed de la file.
    ///
    /// Cette ecriture ne touche volontairement ni `calendar_day_source_audits`
    /// ni les artifacts. Elle est appelee seulement apres que le travail
    /// d'ingestion a ete dead-lettere avec
    /// `calendar_native_sources_refused`: si la persistance de l'audit a
    /// echoue, on ne doit surtout pas repasser par cette table avant de
    /// declarer le jour terminal. Le verrou SQLite de la file reste la source
    /// de verite si cette projection admin echoue elle-meme.
    pub(crate) async fn mark_native_ingestion_refusal_after_audit_persist_failure(
        &self,
        run_id: &str,
        local_day: &str,
        now: OffsetDateTime,
    ) -> Result<(CalendarDayState, CalendarDayRun), AppError> {
        crate::features::calendar_logs::operation("mark_native_ingestion_refusal_after_audit_persist_failure", serde_json::json!({"run_id": format!("{:?}", run_id), "local_day": format!("{:?}", local_day), "now": format!("{:?}", now)}), async {

        let now_text = native_calendar_timestamp(now)?;
        let error_code = native::work::CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR;
        let mut tx = self.pool.begin().await?;
        native_ensure_state_tx(&mut tx, local_day, &now_text).await?;
        native_update_run_status_tx(&mut tx, run_id, "error", Some(error_code), &now_text).await?;
        native_mark_terminal_error_tx(
            &mut tx,
            local_day,
            "ingestion",
            run_id,
            error_code,
            &now_text,
        )
        .await?;
        tx.commit().await?;
        let state = self.get_state(local_day).await?.ok_or_else(|| {
            AppError::storage("calendar_native_state_missing_after_refusal_terminal_mark")
        })?;
        let run = self.get_run(run_id).await?.ok_or_else(|| {
            AppError::storage("calendar_native_run_missing_after_refusal_terminal_mark")
        })?;
        Ok((state, run))

}).await
    }

    #[allow(dead_code)]
    pub(crate) async fn get_item_by_id(
        &self,
        id: i64,
    ) -> Result<Option<CalendarDayItem>, AppError> {
        crate::features::calendar_logs::operation(
            "get_item_by_id",
            serde_json::json!({"id": format!("{:?}", id)}),
            async {
                let item = sqlx::query_as::<_, CalendarDayItem>(&calendar_day_item_select(
                    "WHERE id = ?1",
                ))
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
                Ok(item)
            },
        )
        .await
    }

    pub(crate) async fn list_items(
        &self,
        local_day: &str,
    ) -> Result<Vec<CalendarDayItem>, AppError> {
        crate::features::calendar_logs::operation(
            "list_items",
            serde_json::json!({"local_day": format!("{:?}", local_day)}),
            async {
                let items = sqlx::query_as::<_, CalendarDayItem>(&calendar_day_item_select(
                    "WHERE local_day = ?1 ORDER BY event_time ASC, id ASC",
                ))
                .bind(local_day)
                .fetch_all(&self.pool)
                .await?;
                Ok(items)
            },
        )
        .await
    }

    pub(crate) async fn get_summary_by_id(
        &self,
        summary_id: i64,
    ) -> Result<Option<CalendarDaySummary>, AppError> {
        crate::features::calendar_logs::operation(
            "get_summary_by_id",
            serde_json::json!({"summary_id": format!("{:?}", summary_id)}),
            async {
                let item = sqlx::query_as::<_, CalendarDaySummary>(&calendar_day_summary_select(
                    "WHERE id = ?1",
                ))
                .bind(summary_id)
                .fetch_optional(&self.pool)
                .await?;
                Ok(item)
            },
        )
        .await
    }

    async fn latest_summary(
        &self,
        local_day: &str,
    ) -> Result<Option<CalendarDaySummary>, AppError> {
        crate::features::calendar_logs::operation(
            "latest_summary",
            serde_json::json!({"local_day": format!("{:?}", local_day)}),
            async {
                let item = sqlx::query_as::<_, CalendarDaySummary>(&calendar_day_summary_select(
                    "WHERE local_day = ?1 ORDER BY updated_at DESC, id DESC LIMIT 1",
                ))
                .bind(local_day)
                .fetch_optional(&self.pool)
                .await?;
                Ok(item)
            },
        )
        .await
    }

    pub(crate) async fn get_alert_by_id(
        &self,
        alert_id: i64,
    ) -> Result<Option<CalendarDayAlert>, AppError> {
        crate::features::calendar_logs::operation(
            "get_alert_by_id",
            serde_json::json!({"alert_id": format!("{:?}", alert_id)}),
            async {
                let item = sqlx::query_as::<_, CalendarDayAlert>(&calendar_day_alert_select(
                    "WHERE id = ?1",
                ))
                .bind(alert_id)
                .fetch_optional(&self.pool)
                .await?;
                Ok(item)
            },
        )
        .await
    }

    async fn list_due_alerts(&self, now: &str) -> Result<Vec<CalendarDayAlert>, AppError> {
        crate::features::calendar_logs::operation("list_due_alerts", serde_json::json!({"now": format!("{:?}", now)}), async {

        let items = sqlx::query_as::<_, CalendarDayAlert>(&calendar_day_alert_select(
            "WHERE status = 'scheduled' AND scheduled_at <= ?1 ORDER BY scheduled_at ASC, id ASC LIMIT 25",
        ))
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        Ok(items)

}).await
    }

    async fn mark_alert_published(
        &self,
        alert_id: i64,
    ) -> Result<Option<CalendarDayAlert>, AppError> {
        crate::features::calendar_logs::operation(
            "mark_alert_published",
            serde_json::json!({"alert_id": format!("{:?}", alert_id)}),
            async {
                let now = now_rfc3339();
                sqlx::query(
                    r#"
            UPDATE calendar_day_alerts
            SET status = 'published', published_at = ?2, updated_at = ?2
            WHERE id = ?1 AND status = 'scheduled'
            "#,
                )
                .bind(alert_id)
                .bind(&now)
                .execute(&self.pool)
                .await?;
                self.get_alert_by_id(alert_id).await
            },
        )
        .await
    }

    /// Admet un run ingestion et son travail durable dans une seule
    /// transaction. Le reveil `try_send` et les broadcasts interviennent
    /// ensuite, de sorte qu'une queue memoire saturee ne peut jamais perdre
    /// l'intention SQLite.
    pub(crate) async fn admit_native_ingestion(
        &self,
        work: &native::work::CalendarWorkStore,
        local_day: &str,
        reason: &str,
        attempt: u32,
        now: OffsetDateTime,
    ) -> Result<CalendarNativeIngestionAdmission, AppError> {
        crate::features::calendar_logs::operation("admit_native_ingestion", serde_json::json!({"local_day": format!("{:?}", local_day), "reason": format!("{:?}", reason), "attempt": format!("{:?}", attempt), "now": format!("{:?}", now)}), async {

        let now_text = native_calendar_timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        native_ensure_state_tx(&mut tx, local_day, &now_text).await?;

        let existing = sqlx::query_as::<_, CalendarDayRun>(
            r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE local_day = ?1 AND workflow_kind = 'ingestion' AND status = 'pending'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(local_day)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(run) = existing {
            // Une ancienne base peut contenir un run pending issu d'une version
            // precedente sans travail associe. L'admission idempotente repare
            // uniquement cette intention, sans modifier le run ni l'etat.
            let request = native::work::CalendarWorkRequest::try_new(
                native::work::CalendarWorkKind::Ingestion,
                local_day,
                &run.run_id,
            )?;
            let _ = work.admit_in_tx(&mut tx, &request, now).await?;
            tx.commit().await?;
            return Ok(CalendarNativeIngestionAdmission::Existing(run));
        }

        let run_id = format!(
            "calendar-day-ingestion-{local_day}-{}",
            now.unix_timestamp_nanos()
        );
        sqlx::query(
            r#"
            INSERT INTO calendar_day_runs (
                run_id, local_day, workflow_kind, status, reason, attempt,
                started_at, created_at, updated_at
            )
            VALUES (?1, ?2, 'ingestion', 'pending', ?3, ?4, ?5, ?5, ?5)
            "#,
        )
        .bind(&run_id)
        .bind(local_day)
        .bind(reason)
        .bind(i64::from(attempt))
        .bind(&now_text)
        .execute(&mut *tx)
        .await?;
        let request = native::work::CalendarWorkRequest::try_new(
            native::work::CalendarWorkKind::Ingestion,
            local_day,
            &run_id,
        )?;
        let admission = work.admit_in_tx(&mut tx, &request, now).await?;
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_ingestion_status = 'pending',
                last_run_id = ?2,
                last_error = NULL,
                updated_at = ?3
            WHERE local_day = ?1
            "#,
        )
        .bind(local_day)
        .bind(&run_id)
        .bind(&now_text)
        .execute(&mut *tx)
        .await?;
        let run = sqlx::query_as::<_, CalendarDayRun>(
            r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE run_id = ?1
            "#,
        )
        .bind(&run_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        let state = self.get_state(local_day).await?.ok_or_else(|| {
            AppError::storage("calendar_native_state_missing_after_admission_commit")
        })?;
        Ok(CalendarNativeIngestionAdmission::Started {
            state,
            run,
            admission,
        })

}).await
    }

    /// Admet l'intention Summary native avec le changement d'etat associe.
    ///
    /// Le worker d'ingestion cree normalement ce travail dans son commit
    /// canonique. Cette entree couvre uniquement une reparation administrative
    /// ou hebdomadaire lorsque les donnees existent deja dans SQLite.
    pub(crate) async fn admit_native_summary(
        &self,
        work: &native::work::CalendarWorkStore,
        local_day: &str,
        reason: &str,
        attempt: u32,
        now: OffsetDateTime,
    ) -> Result<CalendarNativeSummaryAdmission, AppError> {
        crate::features::calendar_logs::operation("admit_native_summary", serde_json::json!({"local_day": format!("{:?}", local_day), "reason": format!("{:?}", reason), "attempt": format!("{:?}", attempt), "now": format!("{:?}", now)}), async {

        let now_text = native_calendar_timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        native_ensure_state_tx(&mut tx, local_day, &now_text).await?;
        let existing = sqlx::query_as::<_, CalendarDayRun>(
            r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE local_day = ?1 AND workflow_kind = 'summary' AND status = 'pending'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(local_day)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(run) = existing {
            let request = native::work::CalendarWorkRequest::try_new(
                native::work::CalendarWorkKind::Summary,
                local_day,
                &run.run_id,
            )?;
            let _ = work.admit_in_tx(&mut tx, &request, now).await?;
            tx.commit().await?;
            return Ok(CalendarNativeSummaryAdmission::Existing(run));
        }

        let run_id = format!(
            "calendar-day-summary-{local_day}-{}",
            now.unix_timestamp_nanos()
        );
        sqlx::query(
            r#"
            INSERT INTO calendar_day_runs (
                run_id, local_day, workflow_kind, status, reason, attempt,
                started_at, created_at, updated_at
            )
            VALUES (?1, ?2, 'summary', 'pending', ?3, ?4, ?5, ?5, ?5)
            "#,
        )
        .bind(&run_id)
        .bind(local_day)
        .bind(reason)
        .bind(i64::from(attempt))
        .bind(&now_text)
        .execute(&mut *tx)
        .await?;
        let request = native::work::CalendarWorkRequest::try_new(
            native::work::CalendarWorkKind::Summary,
            local_day,
            &run_id,
        )?;
        let admission = work.admit_in_tx(&mut tx, &request, now).await?;
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_summary_status = 'pending',
                last_run_id = ?2,
                last_error = NULL,
                updated_at = ?3
            WHERE local_day = ?1
            "#,
        )
        .bind(local_day)
        .bind(&run_id)
        .bind(&now_text)
        .execute(&mut *tx)
        .await?;
        let run = sqlx::query_as::<_, CalendarDayRun>(
            r#"
            SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                   error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                   started_at, completed_at, created_at, updated_at
            FROM calendar_day_runs
            WHERE run_id = ?1
            "#,
        )
        .bind(&run_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        let state = self.get_state(local_day).await?.ok_or_else(|| {
            AppError::storage("calendar_native_state_missing_after_summary_admission")
        })?;
        Ok(CalendarNativeSummaryAdmission::Started {
            state,
            run,
            admission,
        })

}).await
    }

    /// Commit atomique de l'ingestion Rust et de toutes ses intentions aval.
    ///
    /// Aucune table `calendar_day_*` ne devient visible partiellement: les
    /// audits rediges, items sans snapshot brut, lignes audio, alertes, travaux
    /// durables, resume pending, etats et run sont valides par le meme commit.
    pub(crate) async fn commit_native_ingestion(
        &self,
        work: &native::work::CalendarWorkStore,
        lease: &native::work::CalendarWorkLease,
        run_id: &str,
        draft: &contracts::CalendarIngestionDraft,
        audits: &[native::collectors::CalendarCollectorAudit],
        alert_offsets_secs: &[u64],
        now: OffsetDateTime,
    ) -> Result<CalendarNativeIngestionCommit, AppError> {
        crate::features::calendar_logs::operation("commit_native_ingestion", serde_json::json!({"lease": format!("{:?}", lease), "run_id": format!("{:?}", run_id), "draft": format!("{:?}", draft), "audits": format!("{:?}", audits), "alert_offsets_secs": format!("{:?}", alert_offsets_secs), "now": format!("{:?}", now)}), async {

        if draft.local_day.is_empty() || draft.items.is_empty() {
            return Err(AppError::storage("calendar_native_empty_ingestion_commit"));
        }
        let local_day = draft.local_day.as_str();
        let now_text = native_calendar_timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        if !work.lease_is_current_in_tx(&mut tx, lease, now).await? {
            return Err(AppError::conflict("calendar_native_work_fenced_out"));
        }

        native_ensure_state_tx(&mut tx, local_day, &now_text).await?;
        native_upsert_source_audits_tx(&mut tx, run_id, local_day, audits, &now_text).await?;
        let source_audit_artifact =
            native_insert_source_audit_artifact_tx(&mut tx, run_id, local_day, audits, &now_text)
                .await?;

        let mut persisted = Vec::with_capacity(draft.items.len());
        for item in &draft.items {
            let item = native_upsert_item_tx(&mut tx, local_day, item, &now_text).await?;
            if item.final_importance == "high" {
                native_ensure_item_audio_pending_tx(&mut tx, item.id, &now_text).await?;
                let request = native::work::CalendarWorkRequest::try_new(
                    native::work::CalendarWorkKind::ItemAudio,
                    local_day,
                    format!("item-{}", item.id),
                )?;
                let _ = work.admit_in_tx(&mut tx, &request, now).await?;

                let alerts = native_ensure_alerts_for_high_item_tx(
                    &mut tx,
                    &item,
                    alert_offsets_secs,
                    &now_text,
                )
                .await?;
                for alert in alerts {
                    let request = native::work::CalendarWorkRequest::try_new(
                        native::work::CalendarWorkKind::AlertAudio,
                        local_day,
                        format!("alert-{}", alert.id),
                    )?;
                    let _ = work.admit_in_tx(&mut tx, &request, now).await?;
                }
            }
            persisted.push(item);
        }

        let summary_run = native_ensure_pending_summary_run_tx(
            &mut tx,
            work,
            local_day,
            "native_ingestion_commit",
            now,
            &now_text,
        )
        .await?;
        let content_hash = draft
            .canonical_hash()
            .map_err(|_| AppError::storage("calendar_native_ingestion_hash_failed"))?;
        native_mark_ingestion_ready_tx(&mut tx, local_day, run_id, &content_hash, &now_text)
            .await?;
        native_update_run_status_tx(&mut tx, run_id, "ready", None, &now_text).await?;

        match work.complete_in_tx(&mut tx, lease, now).await? {
            native::work::CalendarWorkLeaseMutation::Applied(_) => {}
            native::work::CalendarWorkLeaseMutation::FencedOut => {
                return Err(AppError::conflict("calendar_native_work_fenced_out"));
            }
        }
        tx.commit().await?;

        let state = self.get_state(local_day).await?.ok_or_else(|| {
            AppError::storage("calendar_native_state_missing_after_ingestion_commit")
        })?;
        let ingestion_run = self.get_run(run_id).await?.ok_or_else(|| {
            AppError::storage("calendar_native_ingestion_run_missing_after_commit")
        })?;
        Ok(CalendarNativeIngestionCommit {
            state,
            ingestion_run,
            summary_run,
            items: persisted,
            source_audit_artifact,
        })

}).await
    }

    /// Commit atomique d'un refus ou echec terminal de collecte native.
    ///
    /// Une protection anti-bot est ainsi memorisee avec le dead-letter avant
    /// que le worker ne rende sa lease: un redemarrage ne refait pas un appel
    /// vers la meme source pour ce run.
    pub(crate) async fn commit_native_terminal_error(
        &self,
        work: &native::work::CalendarWorkStore,
        lease: &native::work::CalendarWorkLease,
        run_id: &str,
        local_day: &str,
        workflow_kind: &str,
        audits: &[native::collectors::CalendarCollectorAudit],
        error_code: &native::work::CalendarWorkErrorCode,
        now: OffsetDateTime,
    ) -> Result<(CalendarDayState, CalendarDayRun), AppError> {
        crate::features::calendar_logs::operation("commit_native_terminal_error", serde_json::json!({"lease": format!("{:?}", lease), "run_id": format!("{:?}", run_id), "local_day": format!("{:?}", local_day), "workflow_kind": format!("{:?}", workflow_kind), "audits": format!("{:?}", audits), "error_code": format!("{:?}", error_code), "now": format!("{:?}", now)}), async {

        self.commit_native_terminal_error_with_diagnostic(
            work,
            lease,
            run_id,
            local_day,
            workflow_kind,
            audits,
            error_code,
            now,
            None,
        )
        .await

}).await
    }

    pub(crate) async fn commit_native_terminal_error_with_diagnostic(
        &self,
        work: &native::work::CalendarWorkStore,
        lease: &native::work::CalendarWorkLease,
        run_id: &str,
        local_day: &str,
        workflow_kind: &str,
        audits: &[native::collectors::CalendarCollectorAudit],
        error_code: &native::work::CalendarWorkErrorCode,
        now: OffsetDateTime,
        diagnostic: Option<&native::diagnostics::CalendarIngestionDiagnostic>,
    ) -> Result<(CalendarDayState, CalendarDayRun), AppError> {
        crate::features::calendar_logs::operation("commit_native_terminal_error_with_diagnostic", serde_json::json!({"lease": format!("{:?}", lease), "run_id": format!("{:?}", run_id), "local_day": format!("{:?}", local_day), "workflow_kind": format!("{:?}", workflow_kind), "audits": format!("{:?}", audits), "error_code": format!("{:?}", error_code), "now": format!("{:?}", now), "diagnostic": format!("{:?}", diagnostic)}), async {

        let now_text = native_calendar_timestamp(now)?;
        let mut tx = self.pool.begin().await?;
        if !work.lease_is_current_in_tx(&mut tx, lease, now).await? {
            return Err(AppError::conflict("calendar_native_work_fenced_out"));
        }
        native_ensure_state_tx(&mut tx, local_day, &now_text).await?;
        if let Some(diagnostic) = diagnostic {
            // La lease courante et la transaction ecrivain serialisent l'unicite sans migration.
            sqlx::query("INSERT INTO calendar_day_run_artifacts (run_id, local_day, kind, label, content_json, created_at) SELECT ?1, ?2, 'error', 'calendar_native_ingestion_diagnostic', ?3, ?4 WHERE NOT EXISTS (SELECT 1 FROM calendar_day_run_artifacts WHERE run_id=?1 AND label='calendar_native_ingestion_diagnostic')")
                .bind(run_id).bind(local_day).bind(serde_json::to_string(diagnostic)?).bind(&now_text)
                .execute(&mut *tx).await?;
        }
        native_upsert_source_audits_tx(&mut tx, run_id, local_day, audits, &now_text).await?;
        if workflow_kind == "ingestion" && !audits.is_empty() {
            let _ = native_insert_source_audit_artifact_tx(
                &mut tx, run_id, local_day, audits, &now_text,
            )
            .await?;
        }
        native_update_run_status_tx(
            &mut tx,
            run_id,
            "error",
            Some(error_code.as_public_code()),
            &now_text,
        )
        .await?;
        native_mark_terminal_error_tx(
            &mut tx,
            local_day,
            workflow_kind,
            run_id,
            error_code.as_public_code(),
            &now_text,
        )
        .await?;
        match work.fail_in_tx(&mut tx, lease, error_code, now).await? {
            native::work::CalendarWorkLeaseMutation::Applied(_) => {}
            native::work::CalendarWorkLeaseMutation::FencedOut => {
                return Err(AppError::conflict("calendar_native_work_fenced_out"));
            }
        }
        tx.commit().await?;
        let state = self.get_state(local_day).await?.ok_or_else(|| {
            AppError::storage("calendar_native_state_missing_after_terminal_commit")
        })?;
        let run = self.get_run(run_id).await?.ok_or_else(|| {
            AppError::storage("calendar_native_run_missing_after_terminal_commit")
        })?;
        Ok((state, run))

}).await
    }

    /// Commit atomique du resume natif, de son travail HLS et de l'ack de la
    /// lease Summary. Le resume ne peut donc jamais rester `generating` sans
    /// intention durable `summary_audio`.
    pub(crate) async fn commit_native_summary(
        &self,
        work: &native::work::CalendarWorkStore,
        lease: &native::work::CalendarWorkLease,
        run_id: &str,
        draft: &contracts::CalendarSummaryDraft,
        model: &str,
        now: OffsetDateTime,
    ) -> Result<CalendarNativeSummaryCommit, AppError> {
        crate::features::calendar_logs::operation("commit_native_summary", serde_json::json!({"lease": format!("{:?}", lease), "run_id": format!("{:?}", run_id), "draft": format!("{:?}", draft), "model": format!("{:?}", model), "now": format!("{:?}", now)}), async {

        let now_text = native_calendar_timestamp(now)?;
        let content_hash = hash_json(&json!({
            "contract": draft.contract_version,
            "localDay": draft.local_day,
            "title": draft.title,
            "summaryText": draft.summary_text,
            "narrationText": draft.narration_text,
            "highItemIds": draft.input_high_item_ids,
        }));
        let mut tx = self.pool.begin().await?;
        if !work.lease_is_current_in_tx(&mut tx, lease, now).await? {
            return Err(AppError::conflict("calendar_native_work_fenced_out"));
        }
        native_ensure_state_tx(&mut tx, &draft.local_day, &now_text).await?;
        let summary =
            native_upsert_summary_tx(&mut tx, draft, &content_hash, model, &now_text).await?;
        let request = native::work::CalendarWorkRequest::try_new(
            native::work::CalendarWorkKind::SummaryAudio,
            &draft.local_day,
            format!("summary-{}", summary.id),
        )?;
        let _ = work.admit_in_tx(&mut tx, &request, now).await?;
        native_mark_summary_generating_tx(&mut tx, &draft.local_day, &summary, &now_text).await?;
        native_update_run_status_tx(&mut tx, run_id, "audio_generating", None, &now_text).await?;
        match work.complete_in_tx(&mut tx, lease, now).await? {
            native::work::CalendarWorkLeaseMutation::Applied(_) => {}
            native::work::CalendarWorkLeaseMutation::FencedOut => {
                return Err(AppError::conflict("calendar_native_work_fenced_out"));
            }
        }
        tx.commit().await?;
        let state = self.get_state(&draft.local_day).await?.ok_or_else(|| {
            AppError::storage("calendar_native_state_missing_after_summary_commit")
        })?;
        let run = self
            .get_run(run_id)
            .await?
            .ok_or_else(|| AppError::storage("calendar_native_summary_run_missing_after_commit"))?;
        Ok(CalendarNativeSummaryCommit {
            state,
            run,
            summary,
        })

}).await
    }
}

/// Formate l'horloge injectee du commit en UTC afin que les comparaisons
/// SQLite restent stables lors d'un restart ou d'un test de recuperation.
fn native_calendar_timestamp(now: OffsetDateTime) -> Result<String, AppError> {
    now.to_offset(time::UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| AppError::storage("calendar_native_timestamp_format_failed"))
}

async fn native_ensure_state_tx(
    tx: &mut Transaction<'_, Sqlite>,
    local_day: &str,
    now: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO calendar_day_state (
            local_day, calendar_ingestion_status, calendar_summary_status, created_at, updated_at
        )
        VALUES (?1, 'missing', 'missing', ?2, ?2)
        "#,
    )
    .bind(local_day)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn native_upsert_source_audits_tx(
    tx: &mut Transaction<'_, Sqlite>,
    run_id: &str,
    local_day: &str,
    audits: &[native::collectors::CalendarCollectorAudit],
    now: &str,
) -> Result<(), AppError> {
    for audit in audits {
        let (outcome, detail) = native_collector_audit_fields(&audit.outcome);
        sqlx::query(
            r#"
            INSERT INTO calendar_day_source_audits (
                run_id, local_day, source, outcome, detail, http_status,
                raw_sha256, raw_length_bytes, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(run_id, source) DO UPDATE SET
                outcome = excluded.outcome,
                detail = excluded.detail,
                http_status = excluded.http_status,
                raw_sha256 = excluded.raw_sha256,
                raw_length_bytes = excluded.raw_length_bytes,
                created_at = excluded.created_at
            "#,
        )
        .bind(run_id)
        .bind(local_day)
        .bind(native_collector_source_name(audit.source))
        .bind(outcome)
        .bind(detail)
        .bind(audit.http_status.map(i64::from))
        .bind(&audit.raw_sha256)
        .bind(
            audit
                .raw_length_bytes
                .map(|value| i64::try_from(value))
                .transpose()
                .map_err(|_| AppError::storage("calendar_native_source_length_overflow"))?,
        )
        .bind(now)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Rend les audits de collecte visibles par l'admin sans remettre le contenu
/// source dans les artifacts. Le format emploie le contrat d'artifact deja
/// consomme par le GET admin et le WebSocket existants.
async fn native_insert_source_audit_artifact_tx(
    tx: &mut Transaction<'_, Sqlite>,
    run_id: &str,
    local_day: &str,
    audits: &[native::collectors::CalendarCollectorAudit],
    now: &str,
) -> Result<CalendarDayRunArtifact, AppError> {
    let sources = audits
        .iter()
        .map(|audit| {
            let (outcome, detail) = native_collector_audit_fields(&audit.outcome);
            json!({
                "source": native_collector_source_name(audit.source),
                "outcome": outcome,
                "detail": detail,
                "httpStatus": audit.http_status,
                "rawSha256": audit.raw_sha256.as_deref(),
                "rawLengthBytes": audit.raw_length_bytes,
            })
        })
        .collect::<Vec<_>>();
    let content_json = serde_json::to_string(&json!({ "sources": sources }))?;
    // L'audit est ecrit juste apres la collecte, avant l'appel IA. Les
    // commits d'ingestion ou d'erreur qui suivent mettent donc a jour le meme
    // artifact plutot que d'en publier plusieurs pour un seul run.
    let updated = sqlx::query(
        r#"
        UPDATE calendar_day_run_artifacts
        SET content_json = ?1,
            created_at = ?2
        WHERE id = (
            SELECT id
            FROM calendar_day_run_artifacts
            WHERE run_id = ?3
              AND local_day = ?4
              AND label = 'calendar_native_source_audits'
            ORDER BY id DESC
            LIMIT 1
        )
        "#,
    )
    .bind(&content_json)
    .bind(now)
    .bind(run_id)
    .bind(local_day)
    .execute(&mut **tx)
    .await?;
    let id = if updated.rows_affected() == 0 {
        sqlx::query(
            r#"
            INSERT INTO calendar_day_run_artifacts (
                run_id, local_day, kind, label, content_json, created_at
            )
            VALUES (?1, ?2, 'sources', 'calendar_native_source_audits', ?3, ?4)
            "#,
        )
        .bind(run_id)
        .bind(local_day)
        .bind(&content_json)
        .bind(now)
        .execute(&mut **tx)
        .await?
        .last_insert_rowid()
    } else {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id
            FROM calendar_day_run_artifacts
            WHERE run_id = ?1
              AND local_day = ?2
              AND label = 'calendar_native_source_audits'
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(run_id)
        .bind(local_day)
        .fetch_one(&mut **tx)
        .await?
    };
    sqlx::query_as::<_, CalendarDayRunArtifact>(
        r#"
        SELECT id, run_id, local_day, kind, label, content_json, created_at
        FROM calendar_day_run_artifacts
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::from)
}

fn native_collector_source_name(
    source: native::collectors::CalendarCollectorSource,
) -> &'static str {
    source.name()
}

fn native_collector_audit_fields(
    outcome: &native::collectors::CalendarCollectorOutcome,
) -> (&'static str, &'static str) {
    match outcome {
        native::collectors::CalendarCollectorOutcome::Covered => ("covered", "covered"),
        native::collectors::CalendarCollectorOutcome::Unavailable(reason) => (
            "unavailable",
            match reason {
                native::collectors::CalendarCollectorUnavailableReason::Timeout => "timeout",
                native::collectors::CalendarCollectorUnavailableReason::Transport => "transport",
                native::collectors::CalendarCollectorUnavailableReason::Redirect => "redirect",
                native::collectors::CalendarCollectorUnavailableReason::HttpStatus => "http_status",
                native::collectors::CalendarCollectorUnavailableReason::BodyTooLarge => {
                    "body_too_large"
                }
                native::collectors::CalendarCollectorUnavailableReason::InvalidEncoding => {
                    "invalid_encoding"
                }
                native::collectors::CalendarCollectorUnavailableReason::MissingTargetDay => {
                    "missing_target_day"
                }
                native::collectors::CalendarCollectorUnavailableReason::InvalidFeed => {
                    "invalid_feed"
                }
            },
        ),
        native::collectors::CalendarCollectorOutcome::Refused(reason) => (
            "refused",
            match reason {
                native::collectors::CalendarCollectorRefusalReason::Unauthorized => "unauthorized",
                native::collectors::CalendarCollectorRefusalReason::Forbidden => "forbidden",
                native::collectors::CalendarCollectorRefusalReason::RateLimited => "rate_limited",
                native::collectors::CalendarCollectorRefusalReason::LegalRestriction => {
                    "legal_restriction"
                }
                native::collectors::CalendarCollectorRefusalReason::Challenge => "challenge",
            },
        ),
    }
}

fn native_importance_name(value: contracts::CalendarImportance) -> &'static str {
    match value {
        contracts::CalendarImportance::Low => "low",
        contracts::CalendarImportance::Medium => "medium",
        contracts::CalendarImportance::High => "high",
    }
}

fn native_category_name(value: contracts::CalendarCategory) -> &'static str {
    match value {
        contracts::CalendarCategory::Macro => "macro",
        contracts::CalendarCategory::Rates => "rates",
        contracts::CalendarCategory::Inflation => "inflation",
        contracts::CalendarCategory::CentralBank => "central_bank",
        contracts::CalendarCategory::Geopolitics => "geopolitics",
        contracts::CalendarCategory::Company => "company",
        contracts::CalendarCategory::Crypto => "crypto",
        contracts::CalendarCategory::Other => "other",
    }
}

/// Projection compacte d'une provenance d'item rendue par les contrats natifs.
///
/// Les corps HTML, les en-tetes, les challenges anti-bot et `raw` n'y figurent
/// jamais. Seules les metadonnees approuvees par la liaison de collecte sont
/// rendues a l'admin existant via `sources_json`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePersistedItemSource {
    source: &'static str,
    external_id: Option<String>,
    source_url: String,
    raw_title: Option<String>,
}

#[derive(sqlx::FromRow)]
struct NativeExistingItemSourceRow {
    source: String,
    external_id: Option<String>,
    source_url: Option<String>,
    raw_title: Option<String>,
}

fn native_persisted_source_name(
    source: &contracts::CalendarSourceKind,
) -> Result<&'static str, AppError> {
    let contracts::CalendarSourceKind::Feed(source) = source;
    Ok(source.name())
}

/// Valide aussi les lignes deja presentes avant de les conserver lors d'une
/// reingestion. Une provenance historique n'est jamais de confiance: si elle
/// ne satisfait plus les memes regles que le contrat natif, elle est retiree
/// au lieu d'etre recopiee dans `sources_json` ou les surfaces admin.
fn native_persisted_item_source(
    local_day: &str,
    source: contracts::CalendarSourceKind,
    external_id: Option<String>,
    source_url: Option<String>,
    raw_title: Option<String>,
) -> Result<NativePersistedItemSource, AppError> {
    let source_name = native_persisted_source_name(&source)?;
    let source_url = source_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::storage("calendar_native_item_provenance_url_missing"))?;
    let canonical_url = Url::parse(source_url)
        .ok()
        .and_then(|url| {
            contracts::CalendarCoveredSourceBinding::for_local_day(local_day, source.clone(), url)
                .ok()
        })
        .ok_or_else(|| AppError::storage("calendar_native_item_provenance_url_invalid"))?;
    let external_id = contracts::project_native_source_external_id(external_id.as_deref())
        .map_err(|_| AppError::storage("calendar_native_item_provenance_external_id_invalid"))?;
    let raw_title = contracts::project_native_source_raw_title(raw_title.as_deref())
        .map_err(|_| AppError::storage("calendar_native_item_provenance_raw_title_invalid"))?;
    Ok(NativePersistedItemSource {
        source: source_name,
        external_id,
        source_url: canonical_url.canonical_url().to_owned(),
        raw_title,
    })
}

fn native_historical_persisted_item_source(
    local_day: &str,
    row: NativeExistingItemSourceRow,
) -> Option<NativePersistedItemSource> {
    let source = contracts::CalendarSourceKind::from_name(&row.source)?;
    native_persisted_item_source(
        local_day,
        source,
        row.external_id,
        row.source_url,
        row.raw_title,
    )
    .ok()
}

/// Union deterministe : une reingestion partiellement couverte met a jour sa
/// source, mais conserve la provenance valide de l'autre collecteur. Le
/// contrat natif admet au plus une provenance par source pour un item.
fn native_merge_persisted_item_sources(
    mut existing: Vec<NativePersistedItemSource>,
    current: Vec<NativePersistedItemSource>,
) -> Vec<NativePersistedItemSource> {
    for source in current {
        if let Some(previous) = existing
            .iter_mut()
            .find(|previous| previous.source == source.source)
        {
            *previous = source;
        } else {
            existing.push(source);
        }
    }
    existing.sort_by_key(|source| {
        native::feeds::CalendarFeedSource::from_name(source.source)
            .and_then(|source| {
                native::feeds::CalendarFeedSource::ALL
                    .iter()
                    .position(|candidate| *candidate == source)
            })
            .unwrap_or(usize::MAX)
    });
    existing
}

/// Projette la provenance typee vers le sous-ensemble explicitement autorise
/// pour SQLite et les routes existantes.
fn native_persisted_item_sources(
    local_day: &str,
    item: &contracts::CalendarItemDraft,
) -> Result<Vec<NativePersistedItemSource>, AppError> {
    if item.sources.is_empty() {
        return Err(AppError::storage("calendar_native_item_provenance_missing"));
    }

    item.sources
        .iter()
        .map(|source| {
            if !matches!(&source.raw, contracts::CalendarCompactValue::Null) {
                return Err(AppError::storage(
                    "calendar_native_item_provenance_raw_not_projected",
                ));
            }
            native_persisted_item_source(
                local_day,
                source.source.clone(),
                source.external_id.clone(),
                source.source_url.clone(),
                source.raw_title.clone(),
            )
        })
        .collect()
}

/// Upsert natif: les champs editoriaux canoniques et les metadonnees de
/// provenance compactes sont ecrits. Les contenus sources restent en memoire
/// et les audits ne conservent que hash, taille et statut rediges.
async fn native_upsert_item_tx(
    tx: &mut Transaction<'_, Sqlite>,
    local_day: &str,
    item: &contracts::CalendarItemDraft,
    now: &str,
) -> Result<CalendarDayItem, AppError> {
    let event_time = normalize_optional(item.event_time.clone());
    let source_title = normalize_whitespace(&item.source_title);
    let translated_title = normalize_whitespace(&item.translated_title);
    let tts_text = normalize_whitespace(&item.tts_text);
    let reasoning = normalize_whitespace(&item.reasoning);
    if source_title.is_empty()
        || translated_title.is_empty()
        || tts_text.is_empty()
        || reasoning.is_empty()
    {
        return Err(AppError::storage(
            "calendar_native_item_visible_text_invalid",
        ));
    }
    let source_importance = native_importance_name(item.source_importance).to_owned();
    let ai_importance = native_importance_name(item.ai_importance).to_owned();
    let final_importance = native_importance_name(item.final_importance).to_owned();
    let category = native_category_name(item.category).to_owned();
    let current_persisted_sources = native_persisted_item_sources(local_day, item)?;
    let content_hash = hash_json(&json!({
        "localDay": local_day,
        "eventTime": event_time,
        "sourceTitle": source_title,
        "translatedTitle": translated_title,
        "ttsText": tts_text,
        "country": item.country,
        "currency": item.currency,
        "forecast": item.forecast_value,
        "previous": item.previous_value,
        "actual": item.actual_value,
        "sourceImportance": source_importance,
        "aiImportance": ai_importance,
        "finalImportance": final_importance,
        "category": category,
        "reasoning": reasoning,
        "confidence": item.confidence,
    }));
    let existing_item_id = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM calendar_day_items WHERE local_day = ?1 AND content_hash = ?2",
    )
    .bind(local_day)
    .bind(&content_hash)
    .fetch_optional(&mut **tx)
    .await?;
    let existing_sources = match existing_item_id {
        Some(item_id) => sqlx::query_as::<_, NativeExistingItemSourceRow>(
            r#"
            SELECT source, external_id, source_url, raw_title
            FROM calendar_day_item_sources
            WHERE calendar_item_id = ?1
            ORDER BY id ASC
            "#,
        )
        .bind(item_id)
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .filter_map(|row| native_historical_persisted_item_source(local_day, row))
        .collect(),
        None => Vec::new(),
    };
    let persisted_sources =
        native_merge_persisted_item_sources(existing_sources, current_persisted_sources);
    let sources_json = serde_json::to_string(&persisted_sources)
        .map_err(|_| AppError::storage("calendar_native_item_provenance_serialize_failed"))?;

    sqlx::query(
        r#"
        INSERT INTO calendar_day_items (
            local_day, event_time, source_title, translated_title, tts_text,
            country, currency, forecast_value, previous_value, actual_value,
            source_importance, ai_importance, final_importance, category,
            reasoning, confidence, content_hash, sources_json, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?19)
        ON CONFLICT(local_day, content_hash) DO UPDATE SET
            event_time = excluded.event_time,
            source_title = excluded.source_title,
            translated_title = excluded.translated_title,
            tts_text = excluded.tts_text,
            country = excluded.country,
            currency = excluded.currency,
            forecast_value = excluded.forecast_value,
            previous_value = excluded.previous_value,
            actual_value = excluded.actual_value,
            source_importance = excluded.source_importance,
            ai_importance = excluded.ai_importance,
            final_importance = excluded.final_importance,
            category = excluded.category,
            reasoning = excluded.reasoning,
            confidence = excluded.confidence,
            sources_json = excluded.sources_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(local_day)
    .bind(&event_time)
    .bind(&source_title)
    .bind(&translated_title)
    .bind(&tts_text)
    .bind(&item.country)
    .bind(&item.currency)
    .bind(&item.forecast_value)
    .bind(&item.previous_value)
    .bind(&item.actual_value)
    .bind(&source_importance)
    .bind(&ai_importance)
    .bind(&final_importance)
    .bind(&category)
    .bind(&reasoning)
    .bind(item.confidence)
    .bind(&content_hash)
    .bind(&sources_json)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    let item = sqlx::query_as::<_, CalendarDayItem>(&calendar_day_item_select(
        "WHERE local_day = ?1 AND content_hash = ?2",
    ))
    .bind(local_day)
    .bind(&content_hash)
    .fetch_one(&mut **tx)
    .await?;
    // La table enfant est reconstituee depuis l'union validee : une source
    // presente dans le rerun est rafraichie, l'autre source couverte par un
    // run precedent est conservee. Aucun `raw_json` historique n'est recopie.
    sqlx::query("DELETE FROM calendar_day_item_sources WHERE calendar_item_id = ?1")
        .bind(item.id)
        .execute(&mut **tx)
        .await?;
    for source in &persisted_sources {
        sqlx::query(
            r#"
            INSERT INTO calendar_day_item_sources (
                calendar_item_id, source, external_id, source_url, raw_title,
                raw_json, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)
            "#,
        )
        .bind(item.id)
        .bind(source.source)
        .bind(&source.external_id)
        .bind(&source.source_url)
        .bind(&source.raw_title)
        .bind(now)
        .execute(&mut **tx)
        .await?;
    }
    Ok(item)
}

async fn native_ensure_item_audio_pending_tx(
    tx: &mut Transaction<'_, Sqlite>,
    item_id: i64,
    now: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO calendar_day_item_audio (
            calendar_item_id, audio_kind, status, created_at, updated_at
        )
        VALUES (?1, 'event_high', 'pending', ?2, ?2)
        ON CONFLICT(calendar_item_id, audio_kind) DO UPDATE SET
            status = CASE
                WHEN calendar_day_item_audio.status = 'audio_ready' THEN calendar_day_item_audio.status
                ELSE 'pending'
            END,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(item_id)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn native_ensure_alerts_for_high_item_tx(
    tx: &mut Transaction<'_, Sqlite>,
    item: &CalendarDayItem,
    offsets_secs: &[u64],
    now: &str,
) -> Result<Vec<CalendarDayAlert>, AppError> {
    let Some(event_time) = parse_rfc3339(item.event_time.as_deref()) else {
        return Ok(Vec::new());
    };
    let mut alerts = Vec::with_capacity(offsets_secs.len());
    for offset_secs in offsets_secs {
        let offset = i64::try_from(*offset_secs)
            .map_err(|_| AppError::storage("calendar_native_alert_offset_overflow"))?;
        let Some(scheduled_at) = event_time.checked_sub(TimeDuration::seconds(offset)) else {
            continue;
        };
        let Some(expires_at) = event_time
            .checked_add(TimeDuration::minutes(15))
            .and_then(|value| value.format(&Rfc3339).ok())
        else {
            continue;
        };
        let scheduled_at = scheduled_at
            .format(&Rfc3339)
            .map_err(|_| AppError::storage("calendar_native_alert_timestamp_format_failed"))?;
        sqlx::query(
            r#"
            INSERT INTO calendar_day_alerts (
                local_day, calendar_item_id, alert_offset_secs, scheduled_at,
                status, expires_at, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, 'pending_audio', ?5, ?6, ?6)
            ON CONFLICT(calendar_item_id, alert_offset_secs) DO UPDATE SET
                scheduled_at = excluded.scheduled_at,
                expires_at = excluded.expires_at,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&item.local_day)
        .bind(item.id)
        .bind(offset)
        .bind(&scheduled_at)
        .bind(&expires_at)
        .bind(now)
        .execute(&mut **tx)
        .await?;
        let alert = sqlx::query_as::<_, CalendarDayAlert>(&calendar_day_alert_select(
            "WHERE calendar_item_id = ?1 AND alert_offset_secs = ?2",
        ))
        .bind(item.id)
        .bind(offset)
        .fetch_one(&mut **tx)
        .await?;
        alerts.push(alert);
    }
    Ok(alerts)
}

async fn native_ensure_pending_summary_run_tx(
    tx: &mut Transaction<'_, Sqlite>,
    work: &native::work::CalendarWorkStore,
    local_day: &str,
    reason: &str,
    now: OffsetDateTime,
    now_text: &str,
) -> Result<CalendarDayRun, AppError> {
    let existing = sqlx::query_as::<_, CalendarDayRun>(
        r#"
        SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
               error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
               started_at, completed_at, created_at, updated_at
        FROM calendar_day_runs
        WHERE local_day = ?1 AND workflow_kind = 'summary' AND status = 'pending'
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(local_day)
    .fetch_optional(&mut **tx)
    .await?;
    let run = match existing {
        Some(run) => run,
        None => {
            let run_id = format!(
                "calendar-day-summary-{local_day}-{}",
                now.unix_timestamp_nanos()
            );
            sqlx::query(
                r#"
                INSERT INTO calendar_day_runs (
                    run_id, local_day, workflow_kind, status, reason, attempt,
                    started_at, created_at, updated_at
                )
                VALUES (?1, ?2, 'summary', 'pending', ?3, 0, ?4, ?4, ?4)
                "#,
            )
            .bind(&run_id)
            .bind(local_day)
            .bind(reason)
            .bind(now_text)
            .execute(&mut **tx)
            .await?;
            sqlx::query_as::<_, CalendarDayRun>(
                r#"
                SELECT id, run_id, local_day, workflow_kind, status, reason, attempt,
                       error_message, n8n_workflow_id, n8n_workflow_name, n8n_execution_id,
                       started_at, completed_at, created_at, updated_at
                FROM calendar_day_runs
                WHERE run_id = ?1
                "#,
            )
            .bind(&run_id)
            .fetch_one(&mut **tx)
            .await?
        }
    };
    let request = native::work::CalendarWorkRequest::try_new(
        native::work::CalendarWorkKind::Summary,
        local_day,
        &run.run_id,
    )?;
    let _ = work.admit_in_tx(tx, &request, now).await?;
    Ok(run)
}

async fn native_mark_ingestion_ready_tx(
    tx: &mut Transaction<'_, Sqlite>,
    local_day: &str,
    run_id: &str,
    content_hash: &str,
    now: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE calendar_day_state
        SET calendar_ingestion_status = 'ready',
            calendar_summary_status = 'pending',
            calendar_ingested_at = ?2,
            calendar_content_hash = ?3,
            last_run_id = ?4,
            last_error = NULL,
            updated_at = ?2
        WHERE local_day = ?1
        "#,
    )
    .bind(local_day)
    .bind(now)
    .bind(content_hash)
    .bind(run_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn native_update_run_status_tx(
    tx: &mut Transaction<'_, Sqlite>,
    run_id: &str,
    status: &str,
    error_code: Option<&str>,
    now: &str,
) -> Result<(), AppError> {
    let completed_at = matches!(status, "ready" | "audio_ready" | "error").then_some(now);
    sqlx::query(
        r#"
        UPDATE calendar_day_runs
        SET status = ?2,
            error_message = ?3,
            completed_at = COALESCE(?4, completed_at),
            updated_at = ?5
        WHERE run_id = ?1
        "#,
    )
    .bind(run_id)
    .bind(status)
    .bind(error_code)
    .bind(completed_at)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn native_mark_terminal_error_tx(
    tx: &mut Transaction<'_, Sqlite>,
    local_day: &str,
    workflow_kind: &str,
    run_id: &str,
    error_code: &str,
    now: &str,
) -> Result<(), AppError> {
    let (ingestion, summary) = match workflow_kind {
        "ingestion" => (Some("error"), None),
        "summary" => (None, Some("error")),
        _ => return Err(AppError::storage("calendar_native_terminal_kind_invalid")),
    };
    if let Some(status) = ingestion {
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_ingestion_status = ?2,
                last_run_id = ?3,
                last_error = ?4,
                updated_at = ?5
            WHERE local_day = ?1
            "#,
        )
        .bind(local_day)
        .bind(status)
        .bind(run_id)
        .bind(error_code)
        .bind(now)
        .execute(&mut **tx)
        .await?;
    }
    if let Some(status) = summary {
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_summary_status = ?2,
                last_run_id = ?3,
                last_error = ?4,
                updated_at = ?5
            WHERE local_day = ?1
            "#,
        )
        .bind(local_day)
        .bind(status)
        .bind(run_id)
        .bind(error_code)
        .bind(now)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn native_upsert_summary_tx(
    tx: &mut Transaction<'_, Sqlite>,
    draft: &contracts::CalendarSummaryDraft,
    content_hash: &str,
    model: &str,
    now: &str,
) -> Result<CalendarDaySummary, AppError> {
    sqlx::query(
        r#"
        INSERT INTO calendar_day_summaries (
            local_day, title, summary_text, narration_text, content_hash, status,
            high_event_count, provider, model, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, 'audio_generating', ?6, 'openrouter', ?7, ?8, ?8)
        ON CONFLICT(local_day, content_hash) DO UPDATE SET
            title = excluded.title,
            summary_text = excluded.summary_text,
            narration_text = excluded.narration_text,
            status = CASE
                WHEN calendar_day_summaries.status = 'audio_ready' THEN 'audio_ready'
                ELSE 'audio_generating'
            END,
            high_event_count = excluded.high_event_count,
            provider = excluded.provider,
            model = excluded.model,
            error_message = NULL,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&draft.local_day)
    .bind(&draft.title)
    .bind(&draft.summary_text)
    .bind(&draft.narration_text)
    .bind(content_hash)
    .bind(
        i64::try_from(draft.input_high_item_count)
            .map_err(|_| AppError::storage("calendar_native_summary_high_item_count_overflow"))?,
    )
    .bind(model)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    sqlx::query_as::<_, CalendarDaySummary>(&calendar_day_summary_select(
        "WHERE local_day = ?1 AND content_hash = ?2",
    ))
    .bind(&draft.local_day)
    .bind(content_hash)
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::from)
}

async fn native_mark_summary_generating_tx(
    tx: &mut Transaction<'_, Sqlite>,
    local_day: &str,
    summary: &CalendarDaySummary,
    now: &str,
) -> Result<(), AppError> {
    let status = if summary.status == "audio_ready" {
        "audio_ready"
    } else {
        "audio_generating"
    };
    sqlx::query(
        r#"
        UPDATE calendar_day_state
        SET calendar_summary_status = ?2,
            summary_content_hash = CASE WHEN ?2 = 'audio_ready' THEN ?3 ELSE summary_content_hash END,
            last_error = NULL,
            updated_at = ?4
        WHERE local_day = ?1
        "#,
    )
    .bind(local_day)
    .bind(status)
    .bind(&summary.content_hash)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayState {
    pub local_day: String,
    pub calendar_ingestion_status: String,
    pub calendar_summary_status: String,
    pub calendar_ingested_at: Option<String>,
    pub calendar_content_hash: Option<String>,
    pub summary_content_hash: Option<String>,
    pub last_run_id: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayRun {
    pub id: i64,
    pub run_id: String,
    pub local_day: String,
    pub workflow_kind: String,
    pub status: String,
    pub reason: Option<String>,
    pub attempt: i64,
    pub error_message: Option<String>,
    pub n8n_workflow_id: Option<String>,
    pub n8n_workflow_name: Option<String>,
    pub n8n_execution_id: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayRunStep {
    pub id: i64,
    pub run_id: String,
    pub local_day: String,
    pub stage: String,
    pub status: String,
    pub message: String,
    pub detail: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayRunArtifact {
    pub id: i64,
    pub run_id: String,
    pub local_day: String,
    pub kind: String,
    pub label: String,
    pub content_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayItem {
    pub id: i64,
    pub local_day: String,
    pub event_time: Option<String>,
    pub source_title: String,
    pub translated_title: String,
    pub tts_text: String,
    pub country: Option<String>,
    pub currency: Option<String>,
    pub forecast_value: Option<String>,
    pub previous_value: Option<String>,
    pub actual_value: Option<String>,
    pub source_importance: Option<String>,
    pub ai_importance: Option<String>,
    pub final_importance: String,
    pub category: String,
    pub reasoning: Option<String>,
    pub confidence: Option<f64>,
    pub content_hash: String,
    pub sources_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayItemAudio {
    pub id: i64,
    pub calendar_item_id: i64,
    pub audio_kind: String,
    pub status: String,
    pub stream_id: Option<String>,
    pub playlist_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDaySummary {
    pub id: i64,
    pub local_day: String,
    pub title: String,
    pub summary_text: String,
    pub narration_text: String,
    pub content_hash: String,
    pub status: String,
    pub high_event_count: i64,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub stream_id: Option<String>,
    pub playlist_url: Option<String>,
    pub playlist_path: Option<String>,
    pub immutable_playlist_url: Option<String>,
    pub current_playlist_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Statut public compact du résumé calendrier diffusé dans le control plane live.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicCalendarDaySummaryStatus {
    pub local_day: String,
    pub status: String,
    pub audio_available: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayAlert {
    pub id: i64,
    pub local_day: String,
    pub calendar_item_id: i64,
    pub alert_offset_secs: i64,
    pub scheduled_at: String,
    pub status: String,
    pub stream_id: Option<String>,
    pub playlist_url: Option<String>,
    pub published_at: Option<String>,
    pub expires_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Evènements temps réel admin du calendrier du jour.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum CalendarDayAdminBroadcastEvent {
    StateUpdated(CalendarDayState),
    RunStarted(CalendarDayRun),
    RunUpdated(CalendarDayRun),
    Step(CalendarDayRunStep),
    Artifact(CalendarDayRunArtifact),
    ItemsUpdated(Vec<CalendarDayItem>),
    SummaryUpdated(CalendarDaySummary),
    AlertUpdated(CalendarDayAlert),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDayTodayResponse {
    local_day: String,
    state: CalendarDayState,
    item_count: usize,
    high_item_count: usize,
    items: Vec<CalendarDayItem>,
    summary: Option<CalendarDaySummaryResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDaySummaryResponse {
    local_day: String,
    status: String,
    summary_id: Option<i64>,
    title: Option<String>,
    summary_text: Option<String>,
    narration_text: Option<String>,
    stream_id: Option<String>,
    playlist_url: Option<String>,
    immutable_playlist_url: Option<String>,
    current_playlist_url: Option<String>,
    high_event_count: i64,
    error_message: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCalendarDayTodayResponse {
    local_day: String,
    state: CalendarDayState,
    latest_run: Option<CalendarDayRun>,
    runs: Vec<CalendarDayRun>,
    has_active_pending_run: bool,
    active_pending_run_id: Option<String>,
    steps: Vec<CalendarDayRunStep>,
    artifacts: Vec<CalendarDayRunArtifact>,
    items: Vec<CalendarDayItem>,
    audios: Vec<CalendarDayItemAudio>,
    summary: Option<CalendarDaySummary>,
    alerts: Vec<CalendarDayAlert>,
    automatic_window_start: String,
    automatic_window_end: String,
    automatic_retry_base_secs: u64,
    automatic_window_status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCalendarDayRunRequest {
    local_day: Option<String>,
    force: Option<bool>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCalendarWeekRunRequest {
    reference_day: Option<String>,
    force: Option<bool>,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCalendarWeekRunResponse {
    week_start: String,
    week_end: String,
    days: Vec<String>,
    triggered_days: Vec<String>,
    skipped_days: Vec<String>,
    mode: Option<String>,
    sequence_id: Option<String>,
    active_day: Option<String>,
    active_run_id: Option<String>,
    queued_days: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarDayLocalDayQuery {
    local_day: Option<String>,
}

#[derive(Debug)]
pub(crate) enum CalendarDayIngestTriggerOutcome {
    Started(CalendarDayRun),
    Skipped { reason: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientAuthCommand {
    Auth { access_token: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
enum AdminCalendarDayWsMessage {
    Hello(AdminCalendarDayWsHello),
    Snapshot(AdminCalendarDayTodayResponse),
    StateUpdated(CalendarDayState),
    RunStarted(CalendarDayRun),
    RunUpdated(CalendarDayRun),
    Step(CalendarDayRunStep),
    Artifact(CalendarDayRunArtifact),
    ItemsUpdated(Vec<CalendarDayItem>),
    SummaryUpdated(CalendarDaySummary),
    AlertUpdated(CalendarDayAlert),
    Heartbeat(AdminCalendarDayWsHeartbeat),
}

#[derive(Debug, Serialize)]
struct AdminCalendarDayWsHello {
    protocol_version: u8,
    server_time: String,
    user_id: String,
}

#[derive(Debug, Serialize)]
struct AdminCalendarDayWsHeartbeat {
    server_time: String,
}

/// Lance le scheduler calendrier du jour.
pub(crate) fn spawn_scheduler(
    state: AppState,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Option<JoinHandle<()>> {
    if !state.config.calendar_day.active() {
        info!("calendar day scheduler disabled");
        return None;
    }

    Some(tokio::spawn(async move {
        let mut interval = tokio_time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut attempt = 0u32;

        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    match changed {
                        Ok(()) if *shutdown_rx.borrow() => {
                            info!("calendar day scheduler received shutdown signal");
                            break;
                        }
                        Ok(()) => {}
                        Err(_) => {
                            info!("calendar day scheduler shutdown channel closed");
                            break;
                        }
                    }
                }
                _ = interval.tick() => {
                    if let Err(error) = scheduler_tick(&state, &mut attempt).await {
                        warn!(%error, "calendar day scheduler tick failed");
                    }
                }
            }
        }
    }))
}

/// Publie les alertes calendrier dont l'heure est atteinte.
pub(crate) fn spawn_alert_scheduler(
    state: AppState,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Option<JoinHandle<()>> {
    if !state.config.calendar_day.active() {
        return None;
    }

    Some(tokio::spawn(async move {
        let mut interval = tokio_time::interval(Duration::from_secs(15));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    match changed {
                        Ok(()) if *shutdown_rx.borrow() => break,
                        Ok(()) => {}
                        Err(_) => break,
                    }
                }
                _ = interval.tick() => {
                    if let Err(error) = publish_due_calendar_alerts(&state).await {
                        warn!(%error, "calendar day alert scheduler failed");
                    }
                }
            }
        }
    }))
}

pub async fn get_calendar_day_today(
    State(state): State<AppState>,
) -> Result<Json<CalendarDayTodayResponse>, AppError> {
    let local_day = current_local_day()?;
    repair_stale_calendar_summary_audio_for_day(&state, &local_day).await?;
    Ok(Json(
        build_calendar_day_today_response(&state, &local_day).await?,
    ))
}

pub async fn get_calendar_day(
    State(state): State<AppState>,
    Query(query): Query<CalendarDayLocalDayQuery>,
) -> Result<Json<CalendarDayTodayResponse>, AppError> {
    let local_day = normalize_optional(query.local_day).unwrap_or(current_local_day()?);
    repair_stale_calendar_summary_audio_for_day(&state, &local_day).await?;
    Ok(Json(
        build_calendar_day_today_response(&state, &local_day).await?,
    ))
}

pub async fn get_calendar_day_today_summary(
    State(state): State<AppState>,
) -> Result<Json<CalendarDaySummaryResponse>, AppError> {
    let local_day = current_local_day()?;
    repair_stale_calendar_summary_audio_for_day(&state, &local_day).await?;
    Ok(Json(
        build_calendar_day_summary_response(&state, &local_day).await?,
    ))
}

pub async fn get_calendar_day_summary(
    State(state): State<AppState>,
    Query(query): Query<CalendarDayLocalDayQuery>,
) -> Result<Json<CalendarDaySummaryResponse>, AppError> {
    let local_day = normalize_optional(query.local_day).unwrap_or(current_local_day()?);
    repair_stale_calendar_summary_audio_for_day(&state, &local_day).await?;
    Ok(Json(
        build_calendar_day_summary_response(&state, &local_day).await?,
    ))
}

pub async fn get_admin_calendar_day_today(
    State(state): State<AppState>,
) -> Result<Json<AdminCalendarDayTodayResponse>, AppError> {
    let local_day = current_local_day()?;
    Ok(Json(
        build_admin_calendar_day_today_response(&state, &local_day).await?,
    ))
}

pub async fn get_admin_calendar_day(
    State(state): State<AppState>,
    Query(query): Query<CalendarDayLocalDayQuery>,
) -> Result<Json<AdminCalendarDayTodayResponse>, AppError> {
    let local_day = normalize_optional(query.local_day).unwrap_or(current_local_day()?);
    Ok(Json(
        build_admin_calendar_day_today_response(&state, &local_day).await?,
    ))
}

pub async fn run_admin_calendar_day_today(
    State(state): State<AppState>,
    Json(request): Json<AdminCalendarDayRunRequest>,
) -> Result<Json<AdminCalendarDayTodayResponse>, AppError> {
    let local_day = normalize_optional(request.local_day.clone()).unwrap_or(current_local_day()?);
    trigger_admin_calendar_day(&state, &local_day, request).await
}

pub async fn run_admin_calendar_day(
    State(state): State<AppState>,
    Json(request): Json<AdminCalendarDayRunRequest>,
) -> Result<Json<AdminCalendarDayTodayResponse>, AppError> {
    let local_day = normalize_optional(request.local_day.clone()).unwrap_or(current_local_day()?);
    trigger_admin_calendar_day(&state, &local_day, request).await
}

pub async fn run_admin_calendar_week(
    State(state): State<AppState>,
    Json(request): Json<AdminCalendarWeekRunRequest>,
) -> Result<Json<AdminCalendarWeekRunResponse>, AppError> {
    let requested_reference_day =
        normalize_optional(request.reference_day).and_then(|value| parse_local_day(&value));
    let reference_day = requested_reference_day.unwrap_or(current_europe_paris_date()?);
    let days = match requested_reference_day {
        Some(_) => business_week_days_for_date(reference_day),
        None if is_business_weekday(reference_day.weekday()) => {
            business_week_days_for_date(reference_day)
        }
        None => next_business_week_days(reference_day),
    };
    let reason =
        normalize_optional(request.reason).unwrap_or_else(|| "manual_admin_week".to_owned());
    let force = request.force.unwrap_or(true);
    let mut skipped_days = Vec::new();
    let mut triggered_days = Vec::new();
    for day in &days {
        let local_day = day.to_string();
        match native::runtime::admit_ingestion(&state, &local_day, &reason, force, 0).await? {
            CalendarDayIngestTriggerOutcome::Started(run) => {
                triggered_days.push(local_day);
                record_calendar_step(
                    &state,
                    &run.run_id,
                    &run.local_day,
                    "calendar_native_week_admitted",
                    "info",
                    "Le jour ouvrable a ete admis dans la file SQLite native.",
                    None,
                )
                .await;
            }
            CalendarDayIngestTriggerOutcome::Skipped { reason } => {
                skipped_days.push(format!("{local_day}:{reason}"));
            }
        }
    }
    Ok(Json(AdminCalendarWeekRunResponse {
        week_start: days
            .first()
            .map(|day| day.to_string())
            .unwrap_or_else(|| reference_day.to_string()),
        week_end: days
            .last()
            .map(|day| day.to_string())
            .unwrap_or_else(|| reference_day.to_string()),
        days: days.iter().map(|day| day.to_string()).collect(),
        triggered_days: triggered_days.clone(),
        skipped_days,
        // Le contrat admin deja publie ne connait que cette valeur historique.
        // L'orchestrateur est bien SQLite durable en interne, mais ne doit pas
        // modifier la forme ni l'enum de la reponse pendant cette migration.
        mode: Some(CALENDAR_WEEK_RUN_PUBLIC_MODE.to_owned()),
        sequence_id: None,
        active_day: None,
        active_run_id: None,
        queued_days: triggered_days,
    }))
}

async fn trigger_admin_calendar_day(
    state: &AppState,
    local_day: &str,
    request: AdminCalendarDayRunRequest,
) -> Result<Json<AdminCalendarDayTodayResponse>, AppError> {
    ensure_business_local_day(local_day)?;
    let reason = normalize_optional(request.reason).unwrap_or_else(|| "manual_admin".to_owned());
    let force = request.force.unwrap_or(true);
    match native::runtime::admit_ingestion(state, local_day, &reason, force, 0).await? {
        CalendarDayIngestTriggerOutcome::Started(_) => {}
        CalendarDayIngestTriggerOutcome::Skipped { reason } if reason.starts_with("pending:") => {
            return Err(AppError::conflict("calendar_day_ingestion_already_running"));
        }
        CalendarDayIngestTriggerOutcome::Skipped { .. } => {}
    }
    Ok(Json(
        build_admin_calendar_day_today_response(state, local_day).await?,
    ))
}

pub async fn ws_admin_calendar_day(
    State(state): State<AppState>,
    Query(query): Query<CalendarDayLocalDayQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_admin_calendar_day_socket(socket, state, query))
}

async fn scheduler_tick(state: &AppState, attempt: &mut u32) -> Result<(), AppError> {
    let (local_date, local_time) = current_europe_paris_date_time()?;
    match weekly_calendar_scheduler_phase(state, local_date.weekday(), local_time)? {
        CalendarWeekSchedulerPhase::Generate => {
            let days = next_business_week_days(local_date)
                .into_iter()
                .map(|day| day.to_string())
                .collect::<Vec<_>>();
            let mut admitted = false;
            for local_day in days {
                if matches!(
                    native::runtime::admit_ingestion(
                        state,
                        &local_day,
                        "weekly_generation",
                        false,
                        *attempt,
                    )
                    .await?,
                    CalendarDayIngestTriggerOutcome::Started(_)
                ) {
                    admitted = true;
                }
            }
            bump_calendar_week_attempt(attempt, state.config.calendar_day.max_retries, admitted);
        }
        CalendarWeekSchedulerPhase::Verify => {
            let days = next_business_week_days(local_date)
                .into_iter()
                .map(|day| day.to_string())
                .collect::<Vec<_>>();
            let mut ingestion_days = Vec::new();
            for local_day in &days {
                if calendar_business_day_needs_ingestion_for_verification(state, local_day).await? {
                    ingestion_days.push(local_day.clone());
                }
            }
            if !ingestion_days.is_empty() {
                for local_day in ingestion_days {
                    let _ = native::runtime::admit_ingestion(
                        state,
                        &local_day,
                        "weekly_verification",
                        true,
                        0,
                    )
                    .await?;
                }
                return Ok(());
            }
            for local_day in &days {
                verify_calendar_business_day_summary(state, local_day).await?;
            }
        }
        CalendarWeekSchedulerPhase::Idle => {}
    }
    Ok(())
}

async fn calendar_business_day_needs_ingestion_for_verification(
    state: &AppState,
    local_day: &str,
) -> Result<bool, AppError> {
    ensure_business_local_day(local_day)?;
    let state_row = state.calendar_day.ensure_state(local_day).await?;
    // Un refus de source est terminal pour le jour : la verification
    // hebdomadaire ne doit jamais transformer une erreur anti-bot redigee en
    // nouvelle tentative HTTP forcee.
    if state_row.last_error.as_deref() == Some(native::work::CALENDAR_SOURCE_REFUSAL_TERMINAL_ERROR)
    {
        return Ok(false);
    }
    let items = state.calendar_day.list_items(local_day).await?;
    Ok(state_row.calendar_ingestion_status != "ready" || items.is_empty())
}

async fn verify_calendar_business_day_summary(
    state: &AppState,
    local_day: &str,
) -> Result<(), AppError> {
    ensure_business_local_day(local_day)?;
    let Some(summary) = state.calendar_day.latest_summary(local_day).await? else {
        let _ = native::runtime::admit_summary(state, local_day, "weekly_verification", true, 0)
            .await?;
        return Ok(());
    };
    if summary.status == "error" || summary.status == "missing" {
        let _ = native::runtime::admit_summary(state, local_day, "weekly_verification", true, 0)
            .await?;
        return Ok(());
    }
    if matches!(summary.status.as_str(), "audio_ready" | "audio_generating") {
        // `audio_generating` peut venir de l'ancien chemin direct Piper/HLS,
        // qui ne possedait pas encore d'intention `SummaryAudio` durable. Il
        // doit donc passer par la meme reparation que le manifeste manquant,
        // sans attendre indefiniment un worker qui n'existe plus.
        if summary.status == "audio_ready"
            && state
                .hls_origin
                .has_calendar_summary_current_manifest(local_day)
        {
            return Ok(());
        }
        repair_stale_calendar_summary_audio(state, &summary).await?;
        return Ok(());
    }
    Ok(())
}

pub(crate) async fn mark_calendar_summary_audio_error(
    state: &AppState,
    local_day: &str,
    summary_id: i64,
    message: &str,
) -> Result<(), AppError> {
    state
        .calendar_day
        .mark_summary_error(summary_id, message)
        .await?;
    if let Some(summary) = state.calendar_day.get_summary_by_id(summary_id).await? {
        emit_admin_event(
            state,
            CalendarDayAdminBroadcastEvent::SummaryUpdated(summary),
        );
    }
    if let Some(state_row) = state.calendar_day.get_state(local_day).await? {
        emit_admin_event(
            state,
            CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
        );
    }
    if let Some(run) = state.calendar_day.latest_run(local_day).await? {
        if run.workflow_kind == "summary" {
            if let Ok(Some(updated)) = state
                .calendar_day
                .update_run_status(&run.run_id, "error", Some(message))
                .await
            {
                emit_admin_event(state, CalendarDayAdminBroadcastEvent::RunUpdated(updated));
            }
            record_calendar_step(
                state,
                &run.run_id,
                local_day,
                "calendar_summary_hls_audio_failed",
                "error",
                "La generation HLS du resume calendrier a echoue.",
                Some(message),
            )
            .await;
        }
    }
    Ok(())
}

async fn repair_stale_calendar_summary_audio_for_day(
    state: &AppState,
    local_day: &str,
) -> Result<(), AppError> {
    let Some(summary) = state.calendar_day.latest_summary(local_day).await? else {
        return Ok(());
    };
    repair_stale_calendar_summary_audio(state, &summary).await
}

async fn repair_stale_calendar_summary_audio(
    state: &AppState,
    summary: &CalendarDaySummary,
) -> Result<(), AppError> {
    if !matches!(summary.status.as_str(), "audio_ready" | "audio_generating") {
        return Ok(());
    }
    if summary.status == "audio_ready" {
        if summary
            .current_playlist_url
            .as_deref()
            .or(summary.playlist_url.as_deref())
            .is_some_and(|url| url.starts_with("/audio/calendar-summary/"))
            && state
                .hls_origin
                .has_calendar_summary_current_manifest(&summary.local_day)
        {
            return Ok(());
        }
        // Le client consomme le pointeur stable `current.m3u8`. Un manifest
        // immutable, ou un clip encore present en memoire, ne suffit donc pas
        // quand ce pointeur manque: la reparation durable le reconstruit avec
        // le meme stream reserve.
    }

    // Sans runtime natif il n'existe aucun consumer de `SummaryAudio`. Garder
    // l'etat `audio_ready` est preferable a un faux `audio_generating` qui ne
    // pourrait jamais etre repris.
    if !state.config.calendar_day.active() {
        warn!(
            summary_id = summary.id,
            "calendar summary HLS repair skipped because the native calendar runtime is disabled"
        );
        return Ok(());
    }

    let narration_text = normalize_whitespace(&summary.narration_text);
    if narration_text.is_empty() {
        mark_calendar_summary_audio_error(
            state,
            &summary.local_day,
            summary.id,
            "calendar summary audio_ready row has no narration",
        )
        .await?;
        return Ok(());
    }
    if !state.config.tts.enabled || !state.config.hls.enabled {
        mark_calendar_summary_audio_error(
            state,
            &summary.local_day,
            summary.id,
            "calendar summary audio repair requires NEWS_TTS_ENABLED=true and NEWS_HLS_ENABLED=true",
        )
        .await?;
        return Ok(());
    }

    let stale_stream_id = summary.stream_id.as_deref().unwrap_or("missing");
    warn!(
        summary_id = summary.id,
        stream_id = stale_stream_id,
        "calendar summary HLS playlist is missing from origin; admitting durable repair"
    );
    admit_calendar_summary_audio_repair_or_keep_ready(state, summary).await
}

async fn admit_calendar_summary_audio_repair_or_keep_ready(
    state: &AppState,
    summary: &CalendarDaySummary,
) -> Result<(), AppError> {
    let work = native::work::CalendarWorkStore::new(state.calendar_day.pool.clone());
    let repaired = match state
        .calendar_day
        .admit_native_summary_audio_repair(&work, summary.id, OffsetDateTime::now_utc())
        .await
    {
        Ok(Some(repaired)) => repaired,
        Ok(None) => return Ok(()),
        Err(AppError::Conflict(message)) if message == "calendar_native_work_queue_full" => {
            // L'admission et le statut sont transactionnels: ici le resume
            // `audio_ready` conserve sa reponse publique. Une ligne legacy
            // deja bloquee en `audio_generating` n'a aucun travail a reprendre;
            // elle devient donc une erreur redigee plutot que de rester
            // indefiniment en cours.
            warn!(
                summary_id = summary.id,
                "calendar summary HLS repair deferred because durable queue is full"
            );
            if summary.status == "audio_generating" {
                mark_calendar_summary_audio_error(
                    state,
                    &summary.local_day,
                    summary.id,
                    "calendar_native_audio_queue_full",
                )
                .await?;
            }
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    emit_admin_event(
        state,
        CalendarDayAdminBroadcastEvent::StateUpdated(repaired.state),
    );
    emit_admin_event(
        state,
        CalendarDayAdminBroadcastEvent::SummaryUpdated(repaired.summary),
    );
    if let Some(run) = repaired.run {
        emit_admin_event(state, CalendarDayAdminBroadcastEvent::RunUpdated(run));
    }
    if repaired.admission.reactivated() {
        info!(
            summary_id = summary.id,
            "calendar summary HLS repair reactivated a durable audio work"
        );
    }
    let _ = state.calendar_day_native.try_notify();
    Ok(())
}

async fn publish_due_calendar_alerts(state: &AppState) -> Result<(), AppError> {
    state
        .calendar_logs
        .scope(serde_json::json!({"workflow_kind":"alerts"}), async {
            crate::features::calendar_logs::operation(
                "publish_due_calendar_alerts",
                serde_json::json!({}),
                async {
                    let now = now_rfc3339();
                    let alerts = state.calendar_day.list_due_alerts(&now).await?;
                    for alert in alerts {
                        if alert.stream_id.is_none() || alert.playlist_url.is_none() {
                            continue;
                        }
                        let Some(updated) =
                            state.calendar_day.mark_alert_published(alert.id).await?
                        else {
                            continue;
                        };
                        let payload = EdgeCalendarAlertReadyPayload {
                            event_id: edge_event_id_for_calendar_alert(updated.id),
                            server_time: now_rfc3339(),
                            calendar_item_id: updated.calendar_item_id,
                            alert_offset_secs: updated.alert_offset_secs.max(0) as u64,
                            scheduled_at: updated.scheduled_at.clone(),
                            stream_id: updated.stream_id.clone().unwrap_or_default(),
                            playlist_url: updated.playlist_url.clone().unwrap_or_default(),
                            priority: "high".to_owned(),
                            expires_at: updated
                                .expires_at
                                .clone()
                                .unwrap_or_else(|| updated.scheduled_at.clone()),
                            published_at: updated.published_at.clone().unwrap_or_else(now_rfc3339),
                        };
                        state.publish_live_metadata(
                            crate::features::live::dto::EdgeLiveMessage::CalendarAlertReady {
                                payload: payload.clone(),
                            },
                        );
                        publish_calendar_alert_ready(&state.config.edge, payload);
                        emit_admin_event(
                            state,
                            CalendarDayAdminBroadcastEvent::AlertUpdated(updated),
                        );
                    }
                    Ok(())
                },
            )
            .await
        })
        .await
}

async fn build_calendar_day_today_response(
    state: &AppState,
    local_day: &str,
) -> Result<CalendarDayTodayResponse, AppError> {
    let calendar_state = state.calendar_day.ensure_state(local_day).await?;
    let items = state.calendar_day.list_items(local_day).await?;
    let high_count = items
        .iter()
        .filter(|item| item.final_importance == "high")
        .count();
    let item_count = items.len();
    Ok(CalendarDayTodayResponse {
        local_day: local_day.to_owned(),
        state: calendar_state,
        item_count,
        high_item_count: high_count,
        items,
        summary: Some(build_calendar_day_summary_response(state, local_day).await?),
    })
}

async fn build_calendar_day_summary_response(
    state: &AppState,
    local_day: &str,
) -> Result<CalendarDaySummaryResponse, AppError> {
    let summary = state.calendar_day.latest_summary(local_day).await?;
    Ok(match summary {
        Some(summary) => CalendarDaySummaryResponse {
            local_day: local_day.to_owned(),
            status: summary.status.clone(),
            summary_id: Some(summary.id),
            title: Some(summary.title),
            summary_text: Some(summary.summary_text),
            narration_text: Some(summary.narration_text),
            stream_id: summary.stream_id,
            playlist_url: summary.playlist_url,
            immutable_playlist_url: summary.immutable_playlist_url,
            current_playlist_url: summary.current_playlist_url,
            high_event_count: summary.high_event_count,
            error_message: summary.error_message,
            updated_at: Some(summary.updated_at),
        },
        None => CalendarDaySummaryResponse {
            local_day: local_day.to_owned(),
            status: "missing".to_owned(),
            summary_id: None,
            title: None,
            summary_text: None,
            narration_text: None,
            stream_id: None,
            playlist_url: None,
            immutable_playlist_url: None,
            current_playlist_url: None,
            high_event_count: 0,
            error_message: None,
            updated_at: None,
        },
    })
}

pub(crate) async fn public_calendar_day_summary_status(
    state: &AppState,
) -> PublicCalendarDaySummaryStatus {
    let snapshot_time = now_rfc3339();
    let local_day = current_local_day().unwrap_or_else(|_| "1970-01-01".to_owned());
    let summary = match state.calendar_day.latest_summary(&local_day).await {
        Ok(summary) => summary,
        Err(error) => {
            warn!(
                %error,
                local_day,
                "failed to build public calendar day summary status"
            );
            return PublicCalendarDaySummaryStatus {
                local_day,
                status: "error".to_owned(),
                audio_available: false,
                updated_at: snapshot_time,
            };
        }
    };

    match summary {
        Some(summary) => {
            let audio_available =
                summary.status == "audio_ready" && calendar_summary_has_audio_playlist(&summary);
            PublicCalendarDaySummaryStatus {
                local_day: summary.local_day,
                status: summary.status,
                audio_available,
                updated_at: summary.updated_at,
            }
        }
        None => PublicCalendarDaySummaryStatus {
            local_day,
            status: "missing".to_owned(),
            audio_available: false,
            updated_at: snapshot_time,
        },
    }
}

fn calendar_summary_has_audio_playlist(summary: &CalendarDaySummary) -> bool {
    summary
        .current_playlist_url
        .as_deref()
        .or(summary.playlist_url.as_deref())
        .or(summary.immutable_playlist_url.as_deref())
        .is_some_and(|url| !url.trim().is_empty())
}

async fn build_admin_calendar_day_today_response(
    state: &AppState,
    local_day: &str,
) -> Result<AdminCalendarDayTodayResponse, AppError> {
    let calendar_state = state.calendar_day.ensure_state(local_day).await?;
    let latest_run = state.calendar_day.latest_run(local_day).await?;
    let runs = state.calendar_day.list_runs(local_day, 20).await?;
    let active_pending_run = runs
        .iter()
        .find(|run| calendar_day_run_is_active_pending(run))
        .map(|run| run.run_id.clone());
    let (steps, mut artifacts) = match latest_run.as_ref() {
        Some(run) => (
            state.calendar_day.list_steps(&run.run_id).await?,
            state.calendar_day.list_artifacts(&run.run_id).await?,
        ),
        None => (Vec::new(), Vec::new()),
    };
    // Le resume devient souvent le dernier run juste apres l'ingestion. Les
    // audits de sources sont rattaches au run d'ingestion et doivent pourtant
    // rester visibles dans le snapshot admin apres une reconnexion WebSocket.
    if let Some(audit) = state
        .calendar_day
        .latest_native_source_audit_artifact(local_day)
        .await?
    {
        if !artifacts.iter().any(|artifact| artifact.id == audit.id) {
            artifacts.push(audit);
            artifacts.sort_by(|left, right| {
                left.created_at
                    .cmp(&right.created_at)
                    .then_with(|| left.id.cmp(&right.id))
            });
        }
    }
    let items = state.calendar_day.list_items(local_day).await?;
    let audios = list_item_audio_for_day(state, local_day).await?;
    let summary = state.calendar_day.latest_summary(local_day).await?;
    let alerts = list_alerts_for_day(state, local_day).await?;
    Ok(AdminCalendarDayTodayResponse {
        local_day: local_day.to_owned(),
        state: calendar_state,
        latest_run,
        runs,
        has_active_pending_run: active_pending_run.is_some(),
        active_pending_run_id: active_pending_run,
        steps,
        artifacts,
        items,
        audios,
        summary,
        alerts,
        automatic_window_start: state.config.calendar_day.schedule_time.clone(),
        automatic_window_end: state.config.calendar_day.retry_end_time.clone(),
        automatic_retry_base_secs: state.config.calendar_day.retry_base_secs,
        automatic_window_status: calendar_auto_window_status(state)?,
    })
}

async fn list_alerts_for_day(
    state: &AppState,
    local_day: &str,
) -> Result<Vec<CalendarDayAlert>, AppError> {
    let alerts = sqlx::query_as::<_, CalendarDayAlert>(&calendar_day_alert_select(
        "WHERE local_day = ?1 ORDER BY scheduled_at ASC, id ASC",
    ))
    .bind(local_day)
    .fetch_all(&state.calendar_day.pool)
    .await?;
    Ok(alerts)
}

async fn list_item_audio_for_day(
    state: &AppState,
    local_day: &str,
) -> Result<Vec<CalendarDayItemAudio>, AppError> {
    let audios = sqlx::query_as::<_, CalendarDayItemAudio>(
        r#"
        SELECT audio.id, audio.calendar_item_id, audio.audio_kind, audio.status,
               audio.stream_id, audio.playlist_url, audio.error_message,
               audio.created_at, audio.updated_at
        FROM calendar_day_item_audio audio
        INNER JOIN calendar_day_items item ON item.id = audio.calendar_item_id
        WHERE item.local_day = ?1
        ORDER BY audio.updated_at DESC, audio.id DESC
        "#,
    )
    .bind(local_day)
    .fetch_all(&state.calendar_day.pool)
    .await?;
    Ok(audios)
}

async fn record_calendar_step(
    state: &AppState,
    run_id: &str,
    local_day: &str,
    stage: &str,
    status: &str,
    message: &str,
    detail: Option<&str>,
) {
    state.calendar_logs.event("calendar.step", &serde_json::json!({"run_id":run_id,"local_day":local_day,"stage":stage,"status":status,"message":message,"detail":detail}));
    match state
        .calendar_day
        .insert_step(run_id, local_day, stage, status, message, detail)
        .await
    {
        Ok(step) => emit_admin_event(state, CalendarDayAdminBroadcastEvent::Step(step)),
        Err(error) => warn!(%error, %run_id, stage, "failed to persist calendar day step"),
    }
}

pub(crate) fn emit_admin_event(state: &AppState, event: CalendarDayAdminBroadcastEvent) {
    let (kind, count) = match &event {
        CalendarDayAdminBroadcastEvent::StateUpdated(_) => ("state_updated", None),
        CalendarDayAdminBroadcastEvent::RunStarted(_) => ("run_started", None),
        CalendarDayAdminBroadcastEvent::RunUpdated(run) => {
            if let Some(summary) = terminal_run_diagnostic(run) {
                state.calendar_logs.event("workflow.final", &summary);
            }
            ("run_updated", None)
        }
        CalendarDayAdminBroadcastEvent::Step(_) => ("step", None),
        CalendarDayAdminBroadcastEvent::Artifact(_) => ("artifact", None),
        CalendarDayAdminBroadcastEvent::ItemsUpdated(items) => ("items_updated", Some(items.len())),
        CalendarDayAdminBroadcastEvent::SummaryUpdated(_) => ("summary_updated", None),
        CalendarDayAdminBroadcastEvent::AlertUpdated(_) => ("alert_updated", None),
    };
    state.calendar_logs.event(
        "calendar.admin_event",
        &serde_json::json!({"kind": kind, "count": count}),
    );
    let _ = state.calendar_day_admin_tx.send(event);
}

fn terminal_run_diagnostic(run: &CalendarDayRun) -> Option<serde_json::Value> {
    if !matches!(run.status.as_str(), "ready" | "error") {
        return None;
    }
    let duration_ms = run.completed_at.as_deref().and_then(|completed| {
        let start = OffsetDateTime::parse(&run.started_at, &Rfc3339).ok()?;
        let end = OffsetDateTime::parse(completed, &Rfc3339).ok()?;
        Some((end - start).whole_milliseconds().max(0))
    });
    Some(serde_json::json!({
        "run_id": run.run_id,
        "local_day": run.local_day,
        "workflow_kind": run.workflow_kind,
        "status": run.status,
        "attempt": run.attempt,
        "reason": run.reason,
        "error_code": run.error_message,
        "duration_ms": duration_ms,
    }))
}

async fn active_calendar_day_pending_run(
    state: &AppState,
    local_day: &str,
    workflow_kind: &str,
) -> Result<Option<CalendarDayRun>, AppError> {
    let Some(run) = state
        .calendar_day
        .pending_run_by_kind(local_day, workflow_kind)
        .await?
    else {
        return Ok(None);
    };
    if calendar_run_age_secs(&run.started_at)
        .map(|age| age >= CALENDAR_DAY_PENDING_RUN_STALE_SECS)
        .unwrap_or(false)
    {
        let message = format!(
            "calendar {workflow_kind} run marked stale after {CALENDAR_DAY_PENDING_RUN_STALE_SECS}s"
        );
        let _ = state
            .calendar_day
            .update_run_status(&run.run_id, "error", Some(&message))
            .await?;
        let state_row = match workflow_kind {
            "ingestion" => {
                state
                    .calendar_day
                    .update_state_status(
                        local_day,
                        Some("error"),
                        None,
                        Some(&run.run_id),
                        Some(&message),
                    )
                    .await?
            }
            "summary" => {
                state
                    .calendar_day
                    .update_state_status(
                        local_day,
                        None,
                        Some("error"),
                        Some(&run.run_id),
                        Some(&message),
                    )
                    .await?
            }
            _ => state.calendar_day.ensure_state(local_day).await?,
        };
        emit_admin_event(
            state,
            CalendarDayAdminBroadcastEvent::StateUpdated(state_row),
        );
        warn!(
            run_id = %run.run_id,
            local_day,
            workflow_kind,
            "calendar day pending run marked stale"
        );
        return Ok(None);
    }
    Ok(Some(run))
}

async fn calendar_retry_backoff_elapsed(
    state: &AppState,
    local_day: &str,
    workflow_kind: &str,
) -> Result<bool, AppError> {
    let Some(run) = state
        .calendar_day
        .latest_run_by_kind(local_day, workflow_kind)
        .await?
    else {
        return Ok(true);
    };
    if run.status != "error" {
        return Ok(true);
    }
    let reference = run
        .completed_at
        .as_deref()
        .unwrap_or(run.updated_at.as_str());
    let Some(age_secs) = calendar_run_age_secs(reference) else {
        return Ok(true);
    };
    Ok(age_secs >= state.config.calendar_day.retry_base_secs as i64)
}

fn calendar_run_age_secs(timestamp: &str) -> Option<i64> {
    let parsed = OffsetDateTime::parse(timestamp, &Rfc3339).ok()?;
    Some((OffsetDateTime::now_utc() - parsed).whole_seconds())
}

fn calendar_day_run_is_active_pending(run: &CalendarDayRun) -> bool {
    run.status == "pending"
        && calendar_run_age_secs(&run.started_at)
            .map(|age| age < CALENDAR_DAY_PENDING_RUN_STALE_SECS)
            .unwrap_or(true)
}

fn bump_calendar_week_attempt(attempt: &mut u32, max_retries: u32, triggered_run: bool) {
    if triggered_run {
        *attempt = (*attempt).saturating_add(1).min(max_retries);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalendarWeekSchedulerPhase {
    Generate,
    Verify,
    Idle,
}

fn current_local_day() -> Result<String, AppError> {
    europe_paris_day_for_utc(OffsetDateTime::now_utc())
        .map(|day| day.to_string())
        .ok_or_else(|| AppError::Config("failed to resolve Europe/Paris local day".to_owned()))
}

fn current_europe_paris_date() -> Result<Date, AppError> {
    current_europe_paris_date_time().map(|(date, _)| date)
}

fn current_europe_paris_date_time() -> Result<(Date, Time), AppError> {
    let now = OffsetDateTime::now_utc();
    let offset = europe_paris_offset_for_utc(now)
        .ok_or_else(|| AppError::Config("failed to resolve Europe/Paris offset".to_owned()))?;
    let local = now.to_offset(offset);
    Ok((local.date(), local.time()))
}

fn calendar_auto_window_status(state: &AppState) -> Result<String, AppError> {
    calendar_auto_window_status_for_time(state, current_europe_paris_time()?)
}

fn weekly_calendar_scheduler_phase(
    state: &AppState,
    weekday: Weekday,
    local_time: Time,
) -> Result<CalendarWeekSchedulerPhase, AppError> {
    if weekday == Weekday::Saturday
        && is_inside_named_window(
            &state.config.calendar_day.week_generation_window_start,
            &state.config.calendar_day.week_generation_window_end,
            local_time,
        )?
    {
        return Ok(CalendarWeekSchedulerPhase::Generate);
    }
    if weekday == Weekday::Sunday
        && is_inside_named_window(
            &state.config.calendar_day.week_verification_window_start,
            &state.config.calendar_day.week_verification_window_end,
            local_time,
        )?
    {
        return Ok(CalendarWeekSchedulerPhase::Verify);
    }
    Ok(CalendarWeekSchedulerPhase::Idle)
}

fn is_inside_named_window(start: &str, end: &str, local_time: Time) -> Result<bool, AppError> {
    let start = parse_configured_time(start)?;
    let end = parse_configured_time(end)?;
    if end <= start {
        return Err(AppError::Config(
            "calendar week window end must be after start".to_owned(),
        ));
    }
    Ok(local_time >= start && local_time < end)
}

fn calendar_auto_window_status_for_time(
    state: &AppState,
    local_time: Time,
) -> Result<String, AppError> {
    let schedule = parse_configured_time(&state.config.calendar_day.schedule_time)?;
    let retry_end = parse_configured_time(&state.config.calendar_day.retry_end_time)?;
    if retry_end <= schedule {
        return Err(AppError::Config(
            "FILFINANCE_CALENDAR_DAY_RETRY_END_TIME must be after FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME"
                .to_owned(),
        ));
    }
    if local_time < schedule {
        Ok("before_window".to_owned())
    } else if local_time < retry_end {
        Ok("inside_window".to_owned())
    } else {
        Ok("manual_only".to_owned())
    }
}

fn current_europe_paris_time() -> Result<Time, AppError> {
    current_europe_paris_date_time().map(|(_, time)| time)
}

fn parse_configured_time(value: &str) -> Result<Time, AppError> {
    let (hours, minutes) = value.split_once(':').ok_or_else(|| {
        AppError::Config("FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME must be HH:MM".to_owned())
    })?;
    Time::from_hms(
        hours.parse().map_err(|_| {
            AppError::Config("FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME hour is invalid".to_owned())
        })?,
        minutes.parse().map_err(|_| {
            AppError::Config("FILFINANCE_CALENDAR_DAY_SCHEDULE_TIME minute is invalid".to_owned())
        })?,
        0,
    )
    .map_err(|error| AppError::Config(format!("invalid calendar day schedule time: {error}")))
}

fn parse_local_day(value: &str) -> Option<Date> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = value[0..4].parse::<i32>().ok()?;
    let month = value[5..7].parse::<u8>().ok()?;
    let day = value[8..10].parse::<u8>().ok()?;
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}

fn ensure_business_local_day(local_day: &str) -> Result<(), AppError> {
    let Some(date) = parse_local_day(local_day) else {
        return Err(AppError::Config(format!(
            "invalid calendar local day: {local_day}"
        )));
    };
    if !is_business_weekday(date.weekday()) {
        return Err(AppError::Config(format!(
            "calendar day must be Monday-Friday: {local_day}"
        )));
    }
    Ok(())
}

fn business_week_days_for_date(date: Date) -> Vec<Date> {
    let monday = monday_for_week(date);
    (0..5)
        .filter_map(|offset| add_days(monday, offset))
        .collect()
}

fn next_business_week_days(date: Date) -> Vec<Date> {
    add_days(monday_for_week(date), 7)
        .map(business_week_days_for_date)
        .unwrap_or_default()
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

fn is_business_weekday(weekday: Weekday) -> bool {
    matches!(
        weekday,
        Weekday::Monday
            | Weekday::Tuesday
            | Weekday::Wednesday
            | Weekday::Thursday
            | Weekday::Friday
    )
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

fn hash_json(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(value).unwrap_or_default());
    format!("{:x}", hasher.finalize())
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| normalize_whitespace(&value))
        .filter(|value| !value.is_empty())
}

fn parse_rfc3339(value: Option<&str>) -> Option<OffsetDateTime> {
    value.and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
}

fn calendar_day_item_select(where_clause: &str) -> String {
    format!(
        r#"
        SELECT id, local_day, event_time, source_title, translated_title, tts_text,
               country, currency, forecast_value, previous_value, actual_value,
               source_importance, ai_importance, final_importance, category,
               reasoning, confidence, content_hash, sources_json, created_at, updated_at
        FROM calendar_day_items
        {where_clause}
        "#
    )
}

fn calendar_day_summary_select(where_clause: &str) -> String {
    format!(
        r#"
        SELECT id, local_day, title, summary_text, narration_text, content_hash, status,
               high_event_count, provider, model, stream_id, playlist_url,
               playlist_path, immutable_playlist_url, current_playlist_url,
               error_message, created_at, updated_at
        FROM calendar_day_summaries
        {where_clause}
        "#
    )
}

fn calendar_day_alert_select(where_clause: &str) -> String {
    format!(
        r#"
        SELECT id, local_day, calendar_item_id, alert_offset_secs, scheduled_at, status,
               stream_id, playlist_url, published_at, expires_at, error_message,
               created_at, updated_at
        FROM calendar_day_alerts
        {where_clause}
        "#
    )
}

async fn handle_admin_calendar_day_socket(
    mut socket: WebSocket,
    state: AppState,
    query: CalendarDayLocalDayQuery,
) {
    let user = match authenticate_calendar_day_socket(&mut socket, &state).await {
        Ok(user) => user,
        Err(()) => return,
    };
    let access_token_expires_at = user.access_token_expires_at;
    let local_day = normalize_optional(query.local_day)
        .unwrap_or_else(|| current_local_day().unwrap_or_else(|_| "1970-01-01".to_owned()));
    let (mut sender, mut receiver) = socket.split();
    let completed = auth::complete_before_access_token_expiration(
        access_token_expires_at,
        async {
            let hello = AdminCalendarDayWsMessage::Hello(AdminCalendarDayWsHello {
                protocol_version: CALENDAR_DAY_WS_PROTOCOL_VERSION,
                server_time: now_rfc3339(),
                user_id: user.id.clone(),
            });
            if send_admin_ws_message(&mut sender, hello).await.is_err() {
                return;
            }
            if let Ok(snapshot) = build_admin_calendar_day_today_response(&state, &local_day).await {
                let _ = send_admin_ws_message(
                    &mut sender,
                    AdminCalendarDayWsMessage::Snapshot(snapshot),
                )
                .await;
            }

            let mut events = state.calendar_day_admin_tx.subscribe();
            let mut heartbeat =
                tokio_time::interval(Duration::from_secs(CALENDAR_DAY_WS_HEARTBEAT_SECS));
            heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut subscribed_days = HashSet::new();
            subscribed_days.insert(local_day);

            loop {
                tokio::select! {
                    message = receiver.next() => {
                        match message {
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Ok(_)) => {}
                            Some(Err(_)) => break,
                        }
                    }
                    event = events.recv() => {
                        let Ok(event) = event else {
                            break;
                        };
                        if !calendar_admin_event_matches_days(&event, &subscribed_days) {
                            continue;
                        }
                        if send_admin_ws_message(&mut sender, admin_ws_message_from_event(event)).await.is_err() {
                            break;
                        }
                    }
                    _ = heartbeat.tick() => {
                        if send_admin_ws_message(&mut sender, AdminCalendarDayWsMessage::Heartbeat(AdminCalendarDayWsHeartbeat {
                            server_time: now_rfc3339(),
                        })).await.is_err() {
                            break;
                        }
                    }
                }
            }
        },
    )
    .await;

    if completed.is_none() {
        let _ = sender
            .send(auth::access_token_expired_close_message())
            .await;
    }
}

pub(crate) async fn authenticate_calendar_day_socket(
    socket: &mut WebSocket,
    state: &AppState,
) -> Result<CurrentUser, ()> {
    if !state.config.auth.enabled {
        return Ok(CurrentUser::development());
    }
    let token = match tokio_time::timeout(
        Duration::from_secs(CALENDAR_DAY_WS_AUTH_TIMEOUT_SECS),
        socket.recv(),
    )
    .await
    {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ClientAuthCommand>(&text)
        {
            Ok(ClientAuthCommand::Auth { access_token }) => access_token,
            Err(_) => {
                close_ws(socket, "invalid auth message").await;
                return Err(());
            }
        },
        _ => {
            close_ws(socket, "auth timeout").await;
            return Err(());
        }
    };

    let identity = match auth::authenticate_access_token(&state.config.auth, &token).await {
        Ok(identity) => identity,
        Err(error) => {
            warn!(%error, "calendar day websocket rejected invalid token");
            close_ws(socket, "invalid auth token").await;
            return Err(());
        }
    };
    let user = match auth::sync_current_user_profile(state, &identity).await {
        Ok(user) => user,
        Err(error) => {
            warn!(%error, "calendar day websocket failed to sync profile");
            close_ws(socket, "profile persistence failed").await;
            return Err(());
        }
    };
    if !user.can_access_moderator_routes(&state.config.auth) {
        close_ws(socket, "insufficient role").await;
        return Err(());
    }
    Ok(user)
}

async fn close_ws(socket: &mut WebSocket, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: close_code::POLICY,
            reason: reason.into(),
        })))
        .await;
}

async fn send_admin_ws_message(
    sender: &mut SplitSink<WebSocket, Message>,
    message: AdminCalendarDayWsMessage,
) -> Result<(), ()> {
    let payload = serde_json::to_string(&message).map_err(|_| ())?;
    sender
        .send(Message::Text(payload.into()))
        .await
        .map_err(|_| ())
}

fn admin_ws_message_from_event(event: CalendarDayAdminBroadcastEvent) -> AdminCalendarDayWsMessage {
    match event {
        CalendarDayAdminBroadcastEvent::StateUpdated(value) => {
            AdminCalendarDayWsMessage::StateUpdated(value)
        }
        CalendarDayAdminBroadcastEvent::RunStarted(value) => {
            AdminCalendarDayWsMessage::RunStarted(value)
        }
        CalendarDayAdminBroadcastEvent::RunUpdated(value) => {
            AdminCalendarDayWsMessage::RunUpdated(value)
        }
        CalendarDayAdminBroadcastEvent::Step(value) => AdminCalendarDayWsMessage::Step(value),
        CalendarDayAdminBroadcastEvent::Artifact(value) => {
            AdminCalendarDayWsMessage::Artifact(value)
        }
        CalendarDayAdminBroadcastEvent::ItemsUpdated(value) => {
            AdminCalendarDayWsMessage::ItemsUpdated(value)
        }
        CalendarDayAdminBroadcastEvent::SummaryUpdated(value) => {
            AdminCalendarDayWsMessage::SummaryUpdated(value)
        }
        CalendarDayAdminBroadcastEvent::AlertUpdated(value) => {
            AdminCalendarDayWsMessage::AlertUpdated(value)
        }
    }
}

fn calendar_admin_event_matches_days(
    event: &CalendarDayAdminBroadcastEvent,
    days: &HashSet<String>,
) -> bool {
    let local_day = match event {
        CalendarDayAdminBroadcastEvent::StateUpdated(value) => &value.local_day,
        CalendarDayAdminBroadcastEvent::RunStarted(value)
        | CalendarDayAdminBroadcastEvent::RunUpdated(value) => &value.local_day,
        CalendarDayAdminBroadcastEvent::Step(value) => &value.local_day,
        CalendarDayAdminBroadcastEvent::Artifact(value) => &value.local_day,
        CalendarDayAdminBroadcastEvent::ItemsUpdated(values) => {
            return values
                .first()
                .map(|value| days.contains(&value.local_day))
                .unwrap_or(true);
        }
        CalendarDayAdminBroadcastEvent::SummaryUpdated(value) => &value.local_day,
        CalendarDayAdminBroadcastEvent::AlertUpdated(value) => &value.local_day,
    };
    days.contains(local_day)
}

#[cfg(test)]
mod native_summary_audio_repair_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn final_diagnostic_requires_terminal_business_state_and_uses_persisted_duration() {
        let mut run = CalendarDayRun {
            id: 1,
            run_id: "calendar-run-1".into(),
            local_day: "2026-09-23".into(),
            workflow_kind: "ingestion".into(),
            status: "pending".into(),
            reason: None,
            attempt: 2,
            error_message: None,
            n8n_workflow_id: None,
            n8n_workflow_name: None,
            n8n_execution_id: None,
            started_at: "2026-09-23T12:00:00Z".into(),
            completed_at: Some("2026-09-23T12:00:03Z".into()),
            created_at: "2026-09-23T12:00:00Z".into(),
            updated_at: "2026-09-23T12:00:03Z".into(),
        };
        assert!(terminal_run_diagnostic(&run).is_none());
        run.status = "error".into();
        run.error_message = Some("calendar_native_source_unavailable".into());
        let final_event = terminal_run_diagnostic(&run).expect("terminal run is logged");
        assert_eq!(final_event["status"], "error");
        assert_eq!(final_event["attempt"], 2);
        assert_eq!(final_event["duration_ms"], 3000);
    }

    #[test]
    fn weekly_admin_mode_keeps_the_existing_public_contract() {
        assert_eq!(CALENDAR_WEEK_RUN_PUBLIC_MODE, "sequential_memory");
    }

    #[tokio::test]
    async fn legacy_audio_generating_summary_without_work_is_recovered_and_terminalized() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarDayStore { pool };
        store.migrate().await.expect("calendar schema");
        let local_day = "2026-06-15";
        let now = OffsetDateTime::from_unix_timestamp(1_900_000_000)
            .expect("test timestamp must be representable");
        let now_text = native_calendar_timestamp(now).expect("timestamp must format");
        let run = store
            .insert_run(local_day, "summary", "legacy_direct_audio", 0)
            .await
            .expect("legacy summary run");
        store.ensure_state(local_day).await.expect("calendar state");
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_ingestion_status = 'ready',
                calendar_summary_status = 'audio_generating',
                last_run_id = ?2,
                updated_at = ?3
            WHERE local_day = ?1
            "#,
        )
        .bind(local_day)
        .bind(&run.run_id)
        .bind(&now_text)
        .execute(&store.pool)
        .await
        .expect("legacy state update");
        sqlx::query(
            r#"
            INSERT INTO calendar_day_summaries (
                local_day, title, summary_text, narration_text, content_hash, status,
                high_event_count, provider, model, stream_id, created_at, updated_at
            )
            VALUES (?1, 'Titre', 'Resume', 'Narration', 'legacy-hash', 'audio_generating',
                    0, 'openrouter', 'test', 'legacy-summary-stream-55', ?2, ?2)
            "#,
        )
        .bind(local_day)
        .bind(&now_text)
        .execute(&store.pool)
        .await
        .expect("legacy generating summary");
        let summary_id: i64 = sqlx::query_scalar(
            "SELECT id FROM calendar_day_summaries WHERE content_hash = 'legacy-hash'",
        )
        .fetch_one(&store.pool)
        .await
        .expect("legacy summary id");

        let work = native::work::CalendarWorkStore::new(store.pool.clone());
        let repaired = store
            .admit_native_summary_audio_repair(&work, summary_id, now)
            .await
            .expect("legacy repair admission")
            .expect("legacy generating row must be recovered");
        assert!(repaired.admission.inserted());
        assert_eq!(repaired.summary.status, "audio_generating");
        assert_eq!(repaired.state.calendar_summary_status, "audio_generating");
        assert_eq!(
            repaired.run.as_ref().map(|run| run.status.as_str()),
            Some("audio_generating")
        );
        let pending = work
            .pending(Some(native::work::CalendarWorkKind::SummaryAudio), 4)
            .await
            .expect("pending repaired work");
        assert_eq!(pending.len(), 1);
        assert_eq!(
            pending[0].reserved_stream_id.as_deref(),
            Some("legacy-summary-stream-55")
        );

        let claimed = work
            .claim("repair-test-worker", now, Duration::from_secs(60), 1)
            .await
            .expect("repaired work claim")
            .pop()
            .expect("one repaired work");
        let error_code =
            native::work::CalendarWorkErrorCode::try_new("calendar_native_audio_failed")
                .expect("redacted error code");
        let terminal = store
            .commit_native_audio_terminal_error(
                &work,
                &claimed.lease,
                native::work::CalendarWorkKind::SummaryAudio,
                summary_id,
                &error_code,
                now + TimeDuration::seconds(1),
            )
            .await
            .expect("terminal audio commit");
        assert_eq!(
            terminal
                .summary
                .as_ref()
                .map(|summary| summary.status.as_str()),
            Some("error")
        );
        assert_eq!(
            terminal
                .state
                .as_ref()
                .map(|state| state.calendar_summary_status.as_str()),
            Some("error")
        );
        assert_eq!(
            terminal.run.as_ref().map(|run| run.status.as_str()),
            Some("error")
        );
        let status: String =
            sqlx::query_scalar("SELECT status FROM calendar_native_work WHERE id = ?1")
                .bind(claimed.work.id)
                .fetch_one(&store.pool)
                .await
                .expect("terminal durable work status");
        assert_eq!(status, "dead_letter");
    }

    #[tokio::test]
    async fn summary_repair_queue_capacity_keeps_audio_ready_visible() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarDayStore { pool };
        store.migrate().await.expect("calendar schema");
        let local_day = "2026-06-17";
        let now = OffsetDateTime::from_unix_timestamp(1_900_000_200)
            .expect("test timestamp must be representable");
        let now_text = native_calendar_timestamp(now).expect("timestamp must format");
        store.ensure_state(local_day).await.expect("calendar state");
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_ingestion_status = 'ready',
                calendar_summary_status = 'audio_ready',
                updated_at = ?2
            WHERE local_day = ?1
            "#,
        )
        .bind(local_day)
        .bind(&now_text)
        .execute(&store.pool)
        .await
        .expect("ready state");
        sqlx::query(
            r#"
            INSERT INTO calendar_day_summaries (
                local_day, title, summary_text, narration_text, content_hash, status,
                high_event_count, provider, model, stream_id, created_at, updated_at
            )
            VALUES (?1, 'Titre', 'Resume', 'Narration', 'capacity-hash', 'audio_ready',
                    0, 'openrouter', 'test', 'capacity-summary-stream', ?2, ?2)
            "#,
        )
        .bind(local_day)
        .bind(&now_text)
        .execute(&store.pool)
        .await
        .expect("ready summary");
        let summary_id: i64 = sqlx::query_scalar(
            "SELECT id FROM calendar_day_summaries WHERE content_hash = 'capacity-hash'",
        )
        .fetch_one(&store.pool)
        .await
        .expect("summary id");

        let work = native::work::CalendarWorkStore::with_max_open_work(store.pool.clone(), 1)
            .expect("one-work capacity");
        let blocker = native::work::CalendarWorkRequest::try_new(
            native::work::CalendarWorkKind::Ingestion,
            local_day,
            "capacity-blocker",
        )
        .expect("valid blocker request");
        work.admit(&blocker, now)
            .await
            .expect("blocker must fill the durable capacity");

        assert!(matches!(
            store
                .admit_native_summary_audio_repair(&work, summary_id, now)
                .await,
            Err(AppError::Conflict(message)) if message == "calendar_native_work_queue_full"
        ));
        assert_eq!(
            store
                .get_summary_by_id(summary_id)
                .await
                .expect("summary after rejected repair")
                .expect("summary must remain")
                .status,
            "audio_ready",
            "queue capacity must not expose a false audio_generating state"
        );
        assert_eq!(
            store
                .get_state(local_day)
                .await
                .expect("state after rejected repair")
                .expect("state must remain")
                .calendar_summary_status,
            "audio_ready"
        );
    }

    #[tokio::test]
    async fn concurrent_summary_repair_loser_is_a_noop_not_a_public_error() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarDayStore { pool };
        store.migrate().await.expect("calendar schema");
        let local_day = "2026-06-19";
        let now = OffsetDateTime::from_unix_timestamp(1_900_000_500)
            .expect("test timestamp must be representable");
        let now_text = native_calendar_timestamp(now).expect("timestamp must format");
        store.ensure_state(local_day).await.expect("calendar state");
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_ingestion_status = 'ready',
                calendar_summary_status = 'audio_ready',
                updated_at = ?2
            WHERE local_day = ?1
            "#,
        )
        .bind(local_day)
        .bind(&now_text)
        .execute(&store.pool)
        .await
        .expect("ready state");
        sqlx::query(
            r#"
            INSERT INTO calendar_day_summaries (
                local_day, title, summary_text, narration_text, content_hash, status,
                high_event_count, provider, model, stream_id, created_at, updated_at
            )
            VALUES (?1, 'Titre', 'Resume', 'Narration', 'race-hash', 'audio_ready',
                    0, 'openrouter', 'test', 'race-summary-stream', ?2, ?2)
            "#,
        )
        .bind(local_day)
        .bind(&now_text)
        .execute(&store.pool)
        .await
        .expect("ready summary");
        let summary_id: i64 = sqlx::query_scalar(
            "SELECT id FROM calendar_day_summaries WHERE content_hash = 'race-hash'",
        )
        .fetch_one(&store.pool)
        .await
        .expect("summary id");

        // Simule le winner d'une autre route GET publique entre la lecture et
        // l'UPDATE final. `RAISE(IGNORE)` donne exactement zero ligne affectee
        // sans exposer une erreur SQL ou un faux `audio_generating`.
        sqlx::query(
            r#"
            CREATE TRIGGER ignore_second_summary_audio_repair
            BEFORE UPDATE OF status ON calendar_day_summaries
            WHEN OLD.status = 'audio_ready' AND NEW.status = 'audio_generating'
            BEGIN
                SELECT RAISE(IGNORE);
            END
            "#,
        )
        .execute(&store.pool)
        .await
        .expect("race simulation trigger");

        let work = native::work::CalendarWorkStore::new(store.pool.clone());
        assert!(
            store
                .admit_native_summary_audio_repair(&work, summary_id, now)
                .await
                .expect("the losing repair must be a normal no-op")
                .is_none()
        );
        assert_eq!(
            store
                .get_summary_by_id(summary_id)
                .await
                .expect("summary after losing repair")
                .expect("summary must remain")
                .status,
            "audio_ready"
        );
        assert!(
            work.pending(Some(native::work::CalendarWorkKind::SummaryAudio), 4)
                .await
                .expect("pending work after rolled back loser")
                .is_empty(),
            "the losing transaction must not leave a redundant durable job"
        );
    }

    #[tokio::test]
    async fn native_audio_ready_commit_is_fenced_before_summary_state_is_published() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarDayStore { pool };
        store.migrate().await.expect("calendar schema");
        let local_day = "2026-06-16";
        let now = OffsetDateTime::from_unix_timestamp(1_900_000_100)
            .expect("test timestamp must be representable");
        let now_text = native_calendar_timestamp(now).expect("timestamp must format");
        store.ensure_state(local_day).await.expect("calendar state");
        sqlx::query(
            r#"
            UPDATE calendar_day_state
            SET calendar_ingestion_status = 'ready',
                calendar_summary_status = 'audio_generating',
                updated_at = ?2
            WHERE local_day = ?1
            "#,
        )
        .bind(local_day)
        .bind(&now_text)
        .execute(&store.pool)
        .await
        .expect("generating state");
        sqlx::query(
            r#"
            INSERT INTO calendar_day_summaries (
                local_day, title, summary_text, narration_text, content_hash, status,
                high_event_count, provider, model, stream_id, created_at, updated_at
            )
            VALUES (?1, 'Titre', 'Resume', 'Narration', 'fence-hash', 'audio_generating',
                    0, 'openrouter', 'test', 'fenced-summary-stream', ?2, ?2)
            "#,
        )
        .bind(local_day)
        .bind(&now_text)
        .execute(&store.pool)
        .await
        .expect("generating summary");
        let summary_id: i64 = sqlx::query_scalar(
            "SELECT id FROM calendar_day_summaries WHERE content_hash = 'fence-hash'",
        )
        .fetch_one(&store.pool)
        .await
        .expect("summary id");
        let work = native::work::CalendarWorkStore::new(store.pool.clone());
        store
            .admit_native_summary_audio_repair(&work, summary_id, now)
            .await
            .expect("repair admission");

        let first = work
            .claim("first-worker", now, Duration::from_secs(1), 1)
            .await
            .expect("first claim")
            .pop()
            .expect("first leased work");
        work.recover_expired_leases(now + TimeDuration::seconds(2))
            .await
            .expect("lease recovery");
        let second = work
            .claim(
                "second-worker",
                now + TimeDuration::seconds(2),
                Duration::from_secs(30),
                1,
            )
            .await
            .expect("second claim")
            .pop()
            .expect("reclaimed work");
        let target = || CalendarNativeAudioReadyTarget::Summary {
            summary_id,
            stream_id: "fenced-summary-stream".to_owned(),
            playlist_url: "/audio/calendar-summary/2026-06-16/current.m3u8".to_owned(),
            playlist_path: "calendar-summary/2026-06-16/fenced-summary-stream/playlist.m3u8"
                .to_owned(),
            immutable_playlist_url:
                "/audio/calendar-summary/2026-06-16/fenced-summary-stream/playlist.m3u8".to_owned(),
            current_playlist_url: "/audio/calendar-summary/2026-06-16/current.m3u8".to_owned(),
        };
        assert!(matches!(
            store
                .commit_native_audio_ready(&work, &first.lease, target(), now + TimeDuration::seconds(2))
                .await,
            Err(AppError::Conflict(message)) if message == "calendar_native_work_fenced_out"
        ));
        assert_eq!(
            store
                .get_summary_by_id(summary_id)
                .await
                .expect("summary after fenced commit")
                .expect("summary must remain")
                .status,
            "audio_generating"
        );

        let committed = store
            .commit_native_audio_ready(
                &work,
                &second.lease,
                target(),
                now + TimeDuration::seconds(3),
            )
            .await
            .expect("current lease must publish ready state");
        assert_eq!(
            committed
                .summary
                .as_ref()
                .map(|summary| summary.status.as_str()),
            Some("audio_ready")
        );
        let work_status: String =
            sqlx::query_scalar("SELECT status FROM calendar_native_work WHERE id = ?1")
                .bind(second.work.id)
                .fetch_one(&store.pool)
                .await
                .expect("completed work status");
        assert_eq!(work_status, "completed");
    }

    #[tokio::test]
    async fn refusal_audit_is_durable_before_a_retry_can_recollect_the_source() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarDayStore { pool };
        store.migrate().await.expect("calendar schema");
        let local_day = "2026-06-18";
        let now = OffsetDateTime::from_unix_timestamp(1_900_000_300)
            .expect("test timestamp must be representable");
        let run = store
            .insert_run(local_day, "ingestion", "retryable_openrouter", 0)
            .await
            .expect("ingestion run");
        let work = native::work::CalendarWorkStore::new(store.pool.clone());
        let request = native::work::CalendarWorkRequest::try_new(
            native::work::CalendarWorkKind::Ingestion,
            local_day,
            &run.run_id,
        )
        .expect("ingestion request");
        work.admit(&request, now)
            .await
            .expect("ingestion work admission");
        let claimed = work
            .claim("audit-before-retry", now, Duration::from_secs(60), 1)
            .await
            .expect("ingestion work claim")
            .pop()
            .expect("one ingestion work");
        let refusal = native::collectors::CalendarCollectorAudit {
            source: native::collectors::CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
            outcome: native::collectors::CalendarCollectorOutcome::Refused(
                native::collectors::CalendarCollectorRefusalReason::Forbidden,
            ),
            http_status: Some(403),
            raw_sha256: None,
            raw_length_bytes: None,
        };

        let artifact = store
            .persist_native_source_audits(
                &work,
                &claimed.lease,
                &run.run_id,
                local_day,
                &[refusal],
                now,
            )
            .await
            .expect("refusal audit persistence before OpenRouter retry");
        let artifact_json: Value =
            serde_json::from_str(&artifact.content_json).expect("redacted audit artifact JSON");
        assert_eq!(
            artifact_json["sources"][0]["source"],
            "forex_factory_export"
        );
        assert_eq!(artifact_json["sources"][0]["outcome"], "refused");
        assert_eq!(artifact_json["sources"][0]["httpStatus"], 403);

        let quarantines = store
            .native_collector_quarantines(local_day)
            .await
            .expect("next ingestion claim must read the refusal");
        assert!(matches!(
            quarantines.as_slice(),
            [native::collectors::CalendarCollectorQuarantine {
                source: native::collectors::CalendarCollectorSource::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
                reason: native::collectors::CalendarCollectorRefusalReason::Forbidden,
                http_status: Some(403),
            }]
        ));
    }

    #[tokio::test]
    async fn partial_native_reingestion_preserves_valid_provenance_from_the_other_source() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let store = CalendarDayStore { pool };
        store.migrate().await.expect("calendar schema");
        let local_day = "2026-06-15";
        let now = native_calendar_timestamp(
            OffsetDateTime::from_unix_timestamp(1_900_000_400)
                .expect("test timestamp must be representable"),
        )
        .expect("timestamp must format");
        let bea_url = "https://apps.bea.gov/API/signup/release_dates.json";
        let ff_export_url = "https://nfs.faireconomy.media/ff_calendar_thisweek.json";
        let item = |sources| contracts::CalendarItemDraft {
            event_time: Some("2026-06-15T12:00:00Z".to_owned()),
            source_title: "ECB Rate Decision".to_owned(),
            translated_title: "Decision de taux de la BCE".to_owned(),
            tts_text: "La BCE publie sa decision de taux.".to_owned(),
            country: Some("UE".to_owned()),
            currency: Some("EUR".to_owned()),
            forecast_value: Some("2.15%".to_owned()),
            previous_value: Some("2.15%".to_owned()),
            actual_value: None,
            source_importance: contracts::CalendarImportance::High,
            ai_importance: contracts::CalendarImportance::High,
            final_importance: contracts::CalendarImportance::High,
            category: contracts::CalendarCategory::CentralBank,
            reasoning: "Decision monetaire majeure.".to_owned(),
            confidence: 0.95,
            sources,
        };
        let source = |source: contracts::CalendarSourceKind,
                      external_id: &str,
                      source_url: &str,
                      raw_title: &str| contracts::CalendarItemSourceDraft {
            source,
            external_id: Some(external_id.to_owned()),
            source_url: Some(source_url.to_owned()),
            raw_title: Some(raw_title.to_owned()),
            raw: contracts::CalendarCompactValue::Null,
        };

        let first = item(vec![
            source(
                contracts::CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson),
                "bea-123",
                bea_url,
                "ECB Rate Decision",
            ),
            source(
                contracts::CalendarSourceKind::Feed(crate::features::calendar_day::native::feeds::CalendarFeedSource::ForexFactoryExport),
                "ff_export-456",
                ff_export_url,
                "ECB Rate Decision",
            ),
        ]);
        let mut first_tx = store.pool.begin().await.expect("first transaction");
        native_upsert_item_tx(&mut first_tx, local_day, &first, &now)
            .await
            .expect("first native item upsert");
        first_tx.commit().await.expect("first item commit");

        let second = item(vec![source(
            contracts::CalendarSourceKind::Feed(
                crate::features::calendar_day::native::feeds::CalendarFeedSource::BeaJson,
            ),
            "bea-789",
            bea_url,
            "ECB Rate Decision (updated)",
        )]);
        let mut second_tx = store.pool.begin().await.expect("second transaction");
        native_upsert_item_tx(&mut second_tx, local_day, &second, &now)
            .await
            .expect("partial native item upsert");
        second_tx.commit().await.expect("second item commit");

        let sources_json: String =
            sqlx::query_scalar("SELECT sources_json FROM calendar_day_items WHERE local_day = ?1")
                .bind(local_day)
                .fetch_one(&store.pool)
                .await
                .expect("persisted sources JSON");
        let sources: Value = serde_json::from_str(&sources_json).expect("sources JSON");
        assert_eq!(sources.as_array().map(Vec::len), Some(2));
        assert_eq!(sources[0]["source"], "forex_factory_export");
        assert_eq!(sources[0]["externalId"], "ff_export-456");
        assert_eq!(sources[1]["source"], "bea_json");
        assert_eq!(sources[1]["externalId"], "bea-789");
        let source_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM calendar_day_item_sources WHERE calendar_item_id = (SELECT id FROM calendar_day_items WHERE local_day = ?1)",
        )
        .bind(local_day)
        .fetch_one(&store.pool)
        .await
        .expect("persisted provenance count");
        assert_eq!(source_count, 2);
    }
}
