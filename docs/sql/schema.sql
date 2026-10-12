-- Schema consolide courant de Fil Finance.
-- Source runtime actuelle:
-- filfinance-service/src/features/news/sqlite/schema/migration/*.rs.

CREATE TABLE IF NOT EXISTS schema_migrations (
    name TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

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

CREATE TABLE IF NOT EXISTS daily_summary_scheduler_state (
    state_key TEXT PRIMARY KEY,
    last_news_event_id INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);

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

CREATE TABLE IF NOT EXISTS daily_summary_run_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    local_day TEXT NOT NULL,
    kind TEXT NOT NULL,
    label TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

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

CREATE TABLE IF NOT EXISTS user_profiles (
    auth_user_id TEXT PRIMARY KEY,
    email TEXT,
    role TEXT NOT NULL,
    preferred_tts_voice TEXT,
    theme TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

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
    'ECB''s <string>',
    '<string> de la BCE',
    NULL,
    'en',
    'fr',
    'pattern',
    0,
    100,
    1,
    'full_pipeline',
    'Motif directionnel: ECB''s Nagel -> Nagel de la BCE.',
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE NOT EXISTS (
    SELECT 1
    FROM translation_glossary_entries
    WHERE source_term = 'ECB''s <string>'
      AND translated_term = '<string> de la BCE'
      AND match_mode = 'pattern'
);

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
    '<number>h00',
    '<number>h00',
    '<number:hour>',
    'en',
    'fr',
    'pattern',
    0,
    120,
    1,
    'spoken_only',
    'Diction horaire vocale: 14h00 -> quatorze heures.',
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE NOT EXISTS (
    SELECT 1
    FROM translation_glossary_entries
    WHERE source_term = '<number>h00'
      AND translated_term = '<number>h00'
      AND match_mode = 'pattern'
      AND scope = 'spoken_only'
);

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
    '<number>h<number>',
    '<number>h<number>',
    '<number:hour> <number:minute>',
    'en',
    'fr',
    'pattern',
    0,
    119,
    1,
    'spoken_only',
    'Diction horaire vocale: 12h01 -> douze heures une.',
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE NOT EXISTS (
    SELECT 1
    FROM translation_glossary_entries
    WHERE source_term = '<number>h<number>'
      AND translated_term = '<number>h<number>'
      AND match_mode = 'pattern'
      AND scope = 'spoken_only'
);

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

CREATE INDEX IF NOT EXISTS idx_news_events_source_ingested_at
ON news_events (source, ingested_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_ingested_at
ON news_events (event_kind, ingested_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_source_id_desc
ON news_events (source, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_id_desc
ON news_events (event_kind, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_translations_language_id_desc
ON news_translations (language, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_translations_news_event_id
ON news_translations (news_event_id);

CREATE INDEX IF NOT EXISTS idx_news_events_published_at_desc
ON news_events (COALESCE(published_at, ingested_at) DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_source_published_at_desc
ON news_events (source, COALESCE(published_at, ingested_at) DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_published_at_desc
ON news_events (event_kind, COALESCE(published_at, ingested_at) DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_visible_day
ON news_events (substr(COALESCE(published_at, ingested_at), 1, 10), id);

CREATE INDEX IF NOT EXISTS idx_news_events_retention_day
ON news_events (substr(COALESCE(published_at, event_time, ingested_at), 1, 10), id);

CREATE INDEX IF NOT EXISTS idx_news_translations_language_status_published_at_desc
ON news_translations (language, status, COALESCE(published_at, created_at) DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_translations_retention_day
ON news_translations (substr(COALESCE(published_at, created_at), 1, 10), id);

CREATE INDEX IF NOT EXISTS idx_daily_summaries_local_day_updated_at
ON daily_summaries (local_day, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_daily_news_qualifications_local_day_important
ON daily_news_qualifications (local_day, is_important, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_daily_news_qualifications_news_event_id
ON daily_news_qualifications (news_event_id);

CREATE INDEX IF NOT EXISTS idx_daily_summary_runs_local_day_created_at
ON daily_summary_runs (local_day, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_daily_summary_run_steps_run_id_created_at
ON daily_summary_run_steps (run_id, created_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_daily_summary_run_artifacts_run_id_created_at
ON daily_summary_run_artifacts (run_id, created_at ASC, id ASC);

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

CREATE INDEX IF NOT EXISTS idx_translation_glossary_entries_enabled_priority
ON translation_glossary_entries (enabled, priority DESC, source_term);

CREATE INDEX IF NOT EXISTS idx_translation_glossary_entries_source_term
ON translation_glossary_entries (source_term);

CREATE INDEX IF NOT EXISTS idx_news_blacklist_entries_enabled_term
ON news_blacklist_entries (enabled, term);

CREATE INDEX IF NOT EXISTS idx_news_blacklist_hits_created_at
ON news_blacklist_hits (created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_truncation_audit_created_at
ON news_truncation_audit (created_at DESC, id DESC);
