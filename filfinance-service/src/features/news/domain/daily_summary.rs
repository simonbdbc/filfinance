//! Contrats persistés pour le résumé IA journalier.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Ligne SQLite du résumé journalier enrichi.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersistedDailySummary {
    pub id: i64,
    pub local_day: String,
    pub content_hash: String,
    pub summary_kind: String,
    pub status: String,
    pub title: Option<String>,
    pub daily_context_text: Option<String>,
    pub important_news_summary_text: Option<String>,
    pub summary_text: Option<String>,
    pub narration_text: Option<String>,
    pub stream_id: Option<String>,
    pub playlist_url: Option<String>,
    pub news_count: i64,
    pub important_news_count: i64,
    pub source_urls_json: String,
    pub error_message: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub agent_latency_ms: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Brouillon minimal créé quand un run de résumé est demandé.
#[derive(Debug, Clone)]
pub struct DailySummaryDraft {
    pub local_day: String,
    pub content_hash: String,
    pub summary_kind: String,
    pub news_count: i64,
    pub important_news_count: i64,
    pub source_urls_json: String,
}

/// Ligne SQLite d'une qualification de news du jour par n8n/OpenRouter.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersistedDailyNewsQualification {
    pub id: i64,
    pub local_day: String,
    pub news_event_id: i64,
    pub translation_id: Option<i64>,
    pub category: String,
    pub importance: String,
    pub summary: String,
    pub reasoning: String,
    pub confidence: Option<f64>,
    pub is_important: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub qualified_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Brouillon d'upsert pour une qualification journaliere.
#[derive(Debug, Clone)]
pub struct DailyNewsQualificationDraft {
    pub local_day: String,
    pub news_event_id: i64,
    pub translation_id: Option<i64>,
    pub category: String,
    pub importance: String,
    pub summary: String,
    pub reasoning: String,
    pub confidence: Option<f64>,
    pub is_important: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
}

/// Resultat d'upsert permettant a n8n d'envoyer Telegram sans doublon.
#[derive(Debug, Clone)]
pub struct DailyNewsQualificationUpsert {
    pub qualification: PersistedDailyNewsQualification,
    pub became_important: bool,
}

/// Run admin d'une generation de resume journalier.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersistedDailySummaryRun {
    pub id: i64,
    pub run_id: String,
    pub local_day: String,
    pub summary_id: Option<i64>,
    pub status: String,
    pub reason: String,
    pub forced: bool,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// Brouillon de creation d'un run admin.
#[derive(Debug, Clone)]
pub struct DailySummaryRunDraft {
    pub run_id: String,
    pub local_day: String,
    pub summary_id: Option<i64>,
    pub status: String,
    pub reason: String,
    pub forced: bool,
}

/// Etape horodatee d'un run de resume journalier.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersistedDailySummaryRunStep {
    pub id: i64,
    pub run_id: String,
    pub local_day: String,
    pub stage: String,
    pub status: String,
    pub message: String,
    pub detail: Option<String>,
    pub created_at: String,
}

/// Artifact debug redacte lie a un run de resume journalier.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersistedDailySummaryRunArtifact {
    pub id: i64,
    pub run_id: String,
    pub local_day: String,
    pub kind: String,
    pub label: String,
    pub content: String,
    pub created_at: String,
}

/// Evenement live diffuse vers la page admin du resume journalier.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum DailySummaryAdminBroadcastEvent {
    RunStarted(PersistedDailySummaryRun),
    RunUpdated(PersistedDailySummaryRun),
    Step(PersistedDailySummaryRunStep),
    Artifact(PersistedDailySummaryRunArtifact),
    SummaryUpdated(PersistedDailySummary),
}
