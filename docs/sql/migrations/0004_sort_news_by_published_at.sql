-- Ajoute le tri fonctionnel par date de publication avec fallback technique.

ALTER TABLE news_translations
ADD COLUMN published_at TEXT;

CREATE INDEX IF NOT EXISTS idx_news_events_published_at_desc
ON news_events (COALESCE(published_at, ingested_at) DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_translations_language_status_published_at_desc
ON news_translations (language, status, COALESCE(published_at, created_at) DESC, id DESC);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0004_sort_news_by_published_at');
