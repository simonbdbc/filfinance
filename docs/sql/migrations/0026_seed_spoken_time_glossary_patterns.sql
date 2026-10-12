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
    scope,
    notes,
    created_at,
    updated_at
)
SELECT
    '<number>h00',
    '<number>h00',
    '<number:hour>',
    'en',
    'fr',
    'pattern',
    0,
    120,
    1,
    'spoken_only',
    'Diction horaire vocale: 14h00 -> quatorze heures.',
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE NOT EXISTS (
    SELECT 1
    FROM translation_glossary_entries
    WHERE source_term = '<number>h00'
      AND translated_term = '<number>h00'
      AND match_mode = 'pattern'
      AND scope = 'spoken_only'
);

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
    scope,
    notes,
    created_at,
    updated_at
)
SELECT
    '<number>h<number>',
    '<number>h<number>',
    '<number:hour> <number:minute>',
    'en',
    'fr',
    'pattern',
    0,
    119,
    1,
    'spoken_only',
    'Diction horaire vocale: 12h01 -> douze heures une.',
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE NOT EXISTS (
    SELECT 1
    FROM translation_glossary_entries
    WHERE source_term = '<number>h<number>'
      AND translated_term = '<number>h<number>'
      AND match_mode = 'pattern'
      AND scope = 'spoken_only'
);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0026_seed_spoken_time_glossary_patterns');
