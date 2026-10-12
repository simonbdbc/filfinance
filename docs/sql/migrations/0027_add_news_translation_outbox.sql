-- Outbox durable de traduction. Le consumer Rust reste volontairement inactif
-- dans ce palier : cette migration est additive et compatible rollback code.

CREATE TABLE IF NOT EXISTS news_translation_outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    news_event_id INTEGER NOT NULL,
    target_language TEXT NOT NULL CHECK(length(trim(target_language)) > 0),
    status TEXT NOT NULL CHECK(status IN ('pending', 'leased', 'completed', 'dead_letter')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count >= 0),
    available_at TEXT NOT NULL,
    lease_generation INTEGER NOT NULL DEFAULT 0 CHECK(lease_generation >= 0),
    lease_expires_at TEXT,
    last_error_code TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    UNIQUE(news_event_id, target_language),
    FOREIGN KEY(news_event_id) REFERENCES news_events(id) ON DELETE CASCADE,
    CHECK(
        (status = 'leased' AND lease_expires_at IS NOT NULL)
        OR (status != 'leased' AND lease_expires_at IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_news_translation_outbox_claimable
ON news_translation_outbox (status, available_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_news_translation_outbox_expired_lease
ON news_translation_outbox (status, lease_expires_at ASC, id ASC);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0027_add_news_translation_outbox');
