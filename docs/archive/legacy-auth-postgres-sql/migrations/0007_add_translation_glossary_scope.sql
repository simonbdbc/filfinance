ALTER TABLE public.translation_glossary_entries
ADD COLUMN IF NOT EXISTS scope text NOT NULL DEFAULT 'full_pipeline';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'translation_glossary_entries_scope_check'
    ) THEN
        ALTER TABLE public.translation_glossary_entries
        ADD CONSTRAINT translation_glossary_entries_scope_check
        CHECK (scope IN ('full_pipeline', 'spoken_only'));
    END IF;
END;
$$;
