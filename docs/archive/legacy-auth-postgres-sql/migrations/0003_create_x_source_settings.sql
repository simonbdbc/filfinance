-- Migration Supabase Postgres 0003.
-- Parametres admin du collecteur X, geres par le backend Rust.

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
