CREATE TABLE IF NOT EXISTS public.translation_glossary_entries (
    id bigserial PRIMARY KEY,
    source_term text NOT NULL,
    translated_term text NOT NULL,
    spoken_term text,
    source_language text NOT NULL,
    target_language text NOT NULL,
    match_mode text NOT NULL,
    case_sensitive boolean NOT NULL DEFAULT false,
    priority integer NOT NULL DEFAULT 0,
    enabled boolean NOT NULL DEFAULT true,
    scope text NOT NULL DEFAULT 'full_pipeline',
    notes text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT translation_glossary_entries_languages_check
        CHECK (source_language = 'en' AND target_language = 'fr'),
    CONSTRAINT translation_glossary_entries_match_mode_check
        CHECK (match_mode IN ('exact', 'word_boundary', 'pattern')),
    CONSTRAINT translation_glossary_entries_scope_check
        CHECK (scope IN ('full_pipeline', 'spoken_only')),
    CONSTRAINT translation_glossary_entries_priority_check
        CHECK (priority BETWEEN -10000 AND 10000)
);

CREATE INDEX IF NOT EXISTS idx_translation_glossary_entries_enabled_priority
    ON public.translation_glossary_entries (enabled, priority DESC, source_term);

CREATE INDEX IF NOT EXISTS idx_translation_glossary_entries_source_term
    ON public.translation_glossary_entries (source_term);

CREATE OR REPLACE FUNCTION public.set_translation_glossary_entries_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_translation_glossary_entries_updated_at
    ON public.translation_glossary_entries;

CREATE TRIGGER trg_translation_glossary_entries_updated_at
BEFORE UPDATE ON public.translation_glossary_entries
FOR EACH ROW
EXECUTE FUNCTION public.set_translation_glossary_entries_updated_at();

ALTER TABLE public.translation_glossary_entries ENABLE ROW LEVEL SECURITY;
