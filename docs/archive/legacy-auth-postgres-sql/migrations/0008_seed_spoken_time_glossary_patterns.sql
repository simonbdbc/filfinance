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
    scope,
    notes
)
SELECT
    '<number>h00',
    '<number>h00',
    '<number:hour>',
    'en',
    'fr',
    'pattern',
    false,
    120,
    true,
    'spoken_only',
    'Diction horaire vocale: 14h00 -> quatorze heures.'
WHERE NOT EXISTS (
    SELECT 1
    FROM public.translation_glossary_entries
    WHERE source_term = '<number>h00'
      AND translated_term = '<number>h00'
      AND match_mode = 'pattern'
      AND scope = 'spoken_only'
);

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
    scope,
    notes
)
SELECT
    '<number>h<number>',
    '<number>h<number>',
    '<number:hour> <number:minute>',
    'en',
    'fr',
    'pattern',
    false,
    119,
    true,
    'spoken_only',
    'Diction horaire vocale: 12h01 -> douze heures une.'
WHERE NOT EXISTS (
    SELECT 1
    FROM public.translation_glossary_entries
    WHERE source_term = '<number>h<number>'
      AND translated_term = '<number>h<number>'
      AND match_mode = 'pattern'
      AND scope = 'spoken_only'
);
