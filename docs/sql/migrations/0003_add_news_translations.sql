-- Ajoute les traductions persistantes EN/FR et leurs indexes de base.

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
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(news_event_id, language)
);

CREATE INDEX IF NOT EXISTS idx_news_translations_language_id_desc
ON news_translations (language, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_translations_news_event_id
ON news_translations (news_event_id);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0003_add_news_translations');
