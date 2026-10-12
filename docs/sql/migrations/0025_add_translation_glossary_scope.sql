ALTER TABLE translation_glossary_entries
ADD COLUMN scope TEXT NOT NULL DEFAULT 'full_pipeline';

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0025_add_translation_glossary_scope');
