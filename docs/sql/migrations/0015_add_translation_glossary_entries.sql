CREATE TABLE IF NOT EXISTS translation_glossary_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_term TEXT NOT NULL,
    translated_term TEXT NOT NULL,
    spoken_term TEXT,
    source_language TEXT NOT NULL,
    target_language TEXT NOT NULL,
    match_mode TEXT NOT NULL,
    case_sensitive INTEGER NOT NULL DEFAULT 0,
    priority INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_translation_glossary_entries_enabled_priority
    ON translation_glossary_entries (enabled, priority DESC, source_term);

CREATE INDEX IF NOT EXISTS idx_translation_glossary_entries_source_term
    ON translation_glossary_entries (source_term);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0015_add_translation_glossary_entries');
