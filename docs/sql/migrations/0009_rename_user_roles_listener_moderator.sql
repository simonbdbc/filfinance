-- Renomme les roles applicatifs locaux: reader -> listener, operator -> moderator.

UPDATE user_profiles
SET role = CASE role
    WHEN 'reader' THEN 'listener'
    WHEN 'operator' THEN 'moderator'
    ELSE role
END
WHERE role IN ('reader', 'operator');

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0009_rename_user_roles_listener_moderator');
