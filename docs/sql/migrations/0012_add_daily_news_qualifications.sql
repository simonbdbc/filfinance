-- Ajoute les qualifications journalieres de news produites par n8n/OpenRouter.

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

CREATE INDEX IF NOT EXISTS idx_daily_news_qualifications_local_day_important
ON daily_news_qualifications (local_day, is_important, updated_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_daily_news_qualifications_news_event_id
ON daily_news_qualifications (news_event_id);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0012_add_daily_news_qualifications');
