-- Ajoute les resumes IA journaliers generes hors hot path live.

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

CREATE INDEX IF NOT EXISTS idx_daily_summaries_local_day_updated_at
ON daily_summaries (local_day, updated_at DESC, id DESC);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0011_add_daily_summaries');
