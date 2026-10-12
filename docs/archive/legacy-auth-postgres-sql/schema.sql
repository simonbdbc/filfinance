-- Schema Supabase Postgres cible pour les donnees utilisateur Fil Finance.
-- Le pipeline news/audio reste volontairement dans SQLite local.

CREATE TABLE IF NOT EXISTS public.user_profiles (
    supabase_user_id uuid PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    email text,
    role text NOT NULL DEFAULT 'listener',
    preferred_tts_voice text,
    theme text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT user_profiles_role_check CHECK (role IN ('listener', 'moderator', 'admin')),
    CONSTRAINT user_profiles_preferred_tts_voice_length CHECK (
        preferred_tts_voice IS NULL OR length(preferred_tts_voice) <= 128
    ),
    CONSTRAINT user_profiles_theme_check CHECK (
        theme IS NULL OR theme IN ('light', 'dark')
    )
);

CREATE INDEX IF NOT EXISTS idx_user_profiles_email
    ON public.user_profiles (email);

CREATE INDEX IF NOT EXISTS idx_user_profiles_role
    ON public.user_profiles (role);

CREATE OR REPLACE FUNCTION public.set_user_profiles_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_user_profiles_updated_at ON public.user_profiles;

CREATE TRIGGER trg_user_profiles_updated_at
BEFORE UPDATE ON public.user_profiles
FOR EACH ROW
EXECUTE FUNCTION public.set_user_profiles_updated_at();

ALTER TABLE public.user_profiles ENABLE ROW LEVEL SECURITY;

CREATE TABLE IF NOT EXISTS public.x_source_settings (
    id text PRIMARY KEY DEFAULT 'default',
    enabled boolean NOT NULL DEFAULT false,
    account_usernames text[] NOT NULL DEFAULT '{}',
    keywords text[] NOT NULL DEFAULT '{}',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT x_source_settings_singleton_check CHECK (id = 'default')
);

CREATE OR REPLACE FUNCTION public.set_x_source_settings_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_x_source_settings_updated_at ON public.x_source_settings;

CREATE TRIGGER trg_x_source_settings_updated_at
BEFORE UPDATE ON public.x_source_settings
FOR EACH ROW
EXECUTE FUNCTION public.set_x_source_settings_updated_at();

ALTER TABLE public.x_source_settings ENABLE ROW LEVEL SECURITY;

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
    'ECB''s <string>',
    '<string> de la BCE',
    NULL,
    'en',
    'fr',
    'pattern',
    false,
    100,
    true,
    'full_pipeline',
    'Motif directionnel: ECB''s Nagel -> Nagel de la BCE.'
WHERE NOT EXISTS (
    SELECT 1
    FROM public.translation_glossary_entries
    WHERE source_term = 'ECB''s <string>'
      AND translated_term = '<string> de la BCE'
      AND match_mode = 'pattern'
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
