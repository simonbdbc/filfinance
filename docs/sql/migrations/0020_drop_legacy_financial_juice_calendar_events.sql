-- Supprime l'ancien calendrier FinancialJuice legacy.

DROP TABLE IF EXISTS calendar_events;

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0020_drop_legacy_financial_juice_calendar_events');
