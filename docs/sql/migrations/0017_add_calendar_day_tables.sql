-- EPIC-032: calendrier economique du jour n8n + audio Piper.

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
);

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
);

CREATE TABLE IF NOT EXISTS calendar_day_run_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    local_day TEXT NOT NULL,
    stage TEXT NOT NULL,
    status TEXT NOT NULL,
    message TEXT NOT NULL,
    detail TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS calendar_day_run_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    local_day TEXT NOT NULL,
    kind TEXT NOT NULL,
    label TEXT NOT NULL,
    content_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

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
);

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
);

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
);

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
);

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
);

CREATE INDEX IF NOT EXISTS idx_calendar_day_runs_local_day_created_at
ON calendar_day_runs (local_day, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_calendar_day_run_steps_run_id_created_at
ON calendar_day_run_steps (run_id, created_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_calendar_day_run_artifacts_run_id_created_at
ON calendar_day_run_artifacts (run_id, created_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_calendar_day_items_local_day_time
ON calendar_day_items (local_day, event_time ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_calendar_day_items_local_day_importance
ON calendar_day_items (local_day, final_importance, event_time ASC);

CREATE INDEX IF NOT EXISTS idx_calendar_day_summaries_local_day_updated
ON calendar_day_summaries (local_day, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_calendar_day_alerts_due
ON calendar_day_alerts (status, scheduled_at ASC, id ASC);
