-- Blacklist operateur appliquee avant persistence/traduction/audio.
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

CREATE INDEX IF NOT EXISTS idx_news_blacklist_entries_enabled_term
ON news_blacklist_entries (enabled, term);

-- Audit des news ignorees par la blacklist.
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

CREATE INDEX IF NOT EXISTS idx_news_blacklist_hits_created_at
ON news_blacklist_hits (created_at DESC, id DESC);

-- Audit des contenus deja tronques par la source upstream.
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

CREATE INDEX IF NOT EXISTS idx_news_truncation_audit_created_at
ON news_truncation_audit (created_at DESC, id DESC);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0018_add_news_blacklist_and_truncation_audit');
