-- Supprime l'ancienne table de benchmark audio candidats, retiree du runtime actif.

DROP TABLE IF EXISTS news_audio_pipeline_candidates;

INSERT OR IGNORE INTO schema_migrations (name)
VALUES ('0014_drop_news_audio_pipeline_candidates');
