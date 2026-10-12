INSERT INTO public.translation_glossary_entries (
    source_term,
    translated_term,
    spoken_term,
    source_language,
    target_language,
    match_mode,
    case_sensitive,
    priority,
    enabled,
    notes
)
SELECT
    'ECB''s <string>',
    '<string> de la BCE',
    NULL,
    'en',
    'fr',
    'pattern',
    false,
    100,
    true,
    'Motif directionnel: ECB''s Nagel -> Nagel de la BCE.'
WHERE NOT EXISTS (
    SELECT 1
    FROM public.translation_glossary_entries
    WHERE source_term = 'ECB''s <string>'
      AND translated_term = '<string> de la BCE'
      AND match_mode = 'pattern'
);
