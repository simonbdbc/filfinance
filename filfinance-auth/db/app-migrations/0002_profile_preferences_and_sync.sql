ALTER TABLE app.profiles
    ADD COLUMN IF NOT EXISTS preferred_tts_voice text,
    ADD COLUMN IF NOT EXISTS theme text;

ALTER TABLE app.profiles
    DROP CONSTRAINT IF EXISTS profiles_theme_check;

ALTER TABLE app.profiles
    ADD CONSTRAINT profiles_theme_check
    CHECK (theme IS NULL OR theme IN ('light', 'dark'));

CREATE TABLE IF NOT EXISTS app.sync_events (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    profile_id uuid NOT NULL REFERENCES app.profiles(id) ON DELETE CASCADE,
    entity_type text NOT NULL,
    entity_id text NOT NULL,
    operation text NOT NULL CHECK (operation IN ('upsert', 'delete')),
    version bigint NOT NULL,
    payload jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (profile_id, entity_type, entity_id, version)
);

CREATE INDEX IF NOT EXISTS sync_events_profile_cursor_idx
    ON app.sync_events (profile_id, id);
