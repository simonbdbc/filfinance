-- Ajoute les profils utilisateurs locaux et les preferences applicatives.
-- `role` est la source de verite locale des droits Fil Finance.

CREATE TABLE IF NOT EXISTS user_profiles (
    auth_user_id TEXT PRIMARY KEY,
    email TEXT,
    role TEXT NOT NULL,
    preferred_tts_voice TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0007_add_user_profiles');
