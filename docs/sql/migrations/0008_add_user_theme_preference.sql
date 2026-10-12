-- Ajoute la preference de theme utilisateur cote SQLite.
-- SQLite est le store runtime des profils, roles et preferences.

ALTER TABLE user_profiles
ADD COLUMN theme TEXT;

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0008_add_user_theme_preference');
