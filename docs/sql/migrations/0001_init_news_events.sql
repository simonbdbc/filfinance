-- Initialise le registre de migrations et la table canonique des news.

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
    importance TEXT,
    country_code TEXT,
    currency TEXT,
    actual_value TEXT,
    forecast_value TEXT,
    previous_value TEXT,
    raw_payload_ref TEXT,
    signature TEXT NOT NULL UNIQUE
);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0001_init_news_events');
