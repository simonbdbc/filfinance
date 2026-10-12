-- Ajoute les indexes alignes sur les lectures feed et les purges de retention.
CREATE INDEX IF NOT EXISTS idx_news_events_source_published_at_desc
ON news_events (source, COALESCE(published_at, ingested_at) DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_published_at_desc
ON news_events (event_kind, COALESCE(published_at, ingested_at) DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_visible_day
ON news_events (substr(COALESCE(published_at, ingested_at), 1, 10), id);

CREATE INDEX IF NOT EXISTS idx_news_events_retention_day
ON news_events (substr(COALESCE(published_at, event_time, ingested_at), 1, 10), id);

CREATE INDEX IF NOT EXISTS idx_news_translations_retention_day
ON news_translations (substr(COALESCE(published_at, created_at), 1, 10), id);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0024_add_retention_and_feed_query_indexes');
