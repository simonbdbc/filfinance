-- Ajoute le watermark du scheduler de resume IA journalier.

CREATE TABLE IF NOT EXISTS daily_summary_scheduler_state (
    state_key TEXT PRIMARY KEY,
    last_news_event_id INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0013_add_daily_summary_scheduler_state');
