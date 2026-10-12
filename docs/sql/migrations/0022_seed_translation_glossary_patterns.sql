INSERT INTO translation_glossary_entries (
    source_term,
    translated_term,
    spoken_term,
    source_language,
    target_language,
    match_mode,
    case_sensitive,
    priority,
    enabled,
    notes,
    created_at,
    updated_at
)
SELECT
    'ECB''s <string>',
    '<string> de la BCE',
    NULL,
    'en',
    'fr',
    'pattern',
    0,
    100,
    1,
    'Motif directionnel: ECB''s Nagel -> Nagel de la BCE.',
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE NOT EXISTS (
    SELECT 1
    FROM translation_glossary_entries
    WHERE source_term = 'ECB''s <string>'
      AND translated_term = '<string> de la BCE'
      AND match_mode = 'pattern'
);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0022_seed_translation_glossary_patterns');
