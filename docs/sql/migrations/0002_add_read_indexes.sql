-- Ajoute les indexes de lecture rapide sur le flux news.

CREATE INDEX IF NOT EXISTS idx_news_events_source_ingested_at
ON news_events (source, ingested_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_ingested_at
ON news_events (event_kind, ingested_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_source_id_desc
ON news_events (source, id DESC);

CREATE INDEX IF NOT EXISTS idx_news_events_event_kind_id_desc
ON news_events (event_kind, id DESC);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0002_add_read_indexes');
