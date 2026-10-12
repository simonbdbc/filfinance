-- Migration Supabase Postgres 0001.
-- Cree la source de verite distante des profils, roles et preferences utilisateur.

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
