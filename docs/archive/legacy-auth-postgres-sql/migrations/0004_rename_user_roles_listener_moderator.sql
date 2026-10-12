-- Migration Supabase Postgres 0004.
-- Renomme les roles applicatifs: reader -> listener, operator -> moderator.

ALTER TABLE public.user_profiles
    DROP CONSTRAINT IF EXISTS user_profiles_role_check;

UPDATE public.user_profiles
SET role = CASE role
    WHEN 'reader' THEN 'listener'
    WHEN 'operator' THEN 'moderator'
    ELSE role
END
WHERE role IN ('reader', 'operator');

ALTER TABLE public.user_profiles
    ALTER COLUMN role SET DEFAULT 'listener';

ALTER TABLE public.user_profiles
    ADD CONSTRAINT user_profiles_role_check
    CHECK (role IN ('listener', 'moderator', 'admin'));
