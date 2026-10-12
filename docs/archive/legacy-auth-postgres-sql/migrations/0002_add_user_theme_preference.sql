-- Migration Supabase Postgres 0002.
-- Ajoute la preference de theme sans recreer les profils existants.

ALTER TABLE public.user_profiles
ADD COLUMN IF NOT EXISTS theme text;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'user_profiles_theme_check'
          AND conrelid = 'public.user_profiles'::regclass
    ) THEN
        ALTER TABLE public.user_profiles
        ADD CONSTRAINT user_profiles_theme_check
        CHECK (theme IS NULL OR theme IN ('light', 'dark'));
    END IF;
END;
$$;
