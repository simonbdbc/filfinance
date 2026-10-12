-- Ajoute la portee titre/contenu des regles blacklist et l'action d'audit.
ALTER TABLE news_blacklist_entries
ADD COLUMN target_scope TEXT NOT NULL DEFAULT 'title_content';

ALTER TABLE news_blacklist_hits
ADD COLUMN target_scope TEXT NOT NULL DEFAULT 'title_content';

ALTER TABLE news_blacklist_hits
ADD COLUMN action TEXT NOT NULL DEFAULT 'reject_news';

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0023_add_news_blacklist_target_scope');
