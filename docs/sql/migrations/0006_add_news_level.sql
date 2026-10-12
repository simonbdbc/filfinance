-- Ajoute le niveau source FinancialJuice (`Level`) normalise en `news_level`.

ALTER TABLE news_events
ADD COLUMN news_level TEXT;

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0006_add_news_level');
