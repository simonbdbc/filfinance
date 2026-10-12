# SQL Fil Finance

Ce dossier documente les schemas SQL utilises ou cibles par `filfinance-service`.

Le stockage applicatif est separe par responsabilite:

- SQLite local stocke le pipeline news/audio.
- SQLite local stocke aussi les parametres operationnels des sources du pipeline.
- PostgreSQL `app.profiles` stocke les profils, roles et preferences utilisateur.

Le runner effectif SQLite reste aujourd'hui `SqliteNewsStore::migrate()` dans:

```text
filfinance-service/src/features/news/sqlite/schema/migration.rs
```

Les fichiers SQL servent de reference lisible pour l'exploitation, les reviews et les futures extractions de migrations.

## Contenu

- `schema.sql`: schema SQLite consolide courant du pipeline local.
- `migrations/`: historique SQLite aligne avec les noms enregistres dans `schema_migrations`.
- `../archive/legacy-auth-postgres-sql/`: archive historique des anciens schemas PostgreSQL externes.

## Ordre des migrations SQLite

1. `0001_init_news_events.sql`
2. `0002_add_read_indexes.sql`
3. `0003_add_news_translations.sql`
4. `0004_sort_news_by_published_at.sql`
5. `0006_add_news_level.sql`
6. `0007_add_user_profiles.sql`
7. `0008_add_user_theme_preference.sql`
8. `0009_rename_user_roles_listener_moderator.sql`
9. `0010_rename_user_profiles_auth_user_id.sql`
10. `0011_add_daily_summaries.sql`
11. `0012_add_daily_news_qualifications.sql`
12. `0013_add_daily_summary_scheduler_state.sql`
13. `0014_drop_news_audio_pipeline_candidates.sql`
14. `0015_add_translation_glossary_entries.sql`
15. `0016_add_daily_summary_admin_runs.sql`
16. `0017_add_calendar_day_tables.sql`
17. `0018_add_news_blacklist_and_truncation_audit.sql`
18. `0019_add_calendar_day_n8n_metadata.sql`
19. `0020_drop_legacy_financial_juice_calendar_events.sql`
20. `0021_add_calendar_summary_hls_manifest_paths.sql`
21. `0022_seed_translation_glossary_patterns.sql`
22. `0023_add_news_blacklist_target_scope.sql`
23. `0024_add_retention_and_feed_query_indexes.sql`
24. `0025_add_translation_glossary_scope.sql`
25. `0026_seed_spoken_time_glossary_patterns.sql`

## Notes d'exploitation

- `news_events` et `news_translations` restent en SQLite pour garder le hot path news/audio local, rapide et independant.
- `daily_news_qualifications`, `daily_summaries`, `daily_summary_scheduler_state`, `daily_summary_runs`, `daily_summary_run_steps` et `daily_summary_run_artifacts` sont conserves comme historique legacy; ils ne portent plus le runtime actif du resume produit.
- `calendar_day_*` stocke le nouveau calendrier economique journalier actif: runs n8n separes, evenements fusionnes, audios Piper, resume et alertes T-5/T-1.
- `migrations/0007_add_user_profiles.sql` represente l'etat transitoire livre dans `NEWS-071`: profils, roles et preference TTS stockes localement.
- `migrations/0008_add_user_theme_preference.sql` ajoute la preference `theme` pour synchroniser le switch clair/sombre.
- `migrations/0009_rename_user_roles_listener_moderator.sql` renomme les roles applicatifs locaux `reader` -> `listener` et `operator` -> `moderator`.
- `migrations/0010_rename_user_profiles_auth_user_id.sql` documente le dernier palier historique des profils SQLite.
- `migrations/0011_add_daily_summaries.sql` ajoute le resume journalier versionne par `local_day + content_hash`.
- `migrations/0012_add_daily_news_qualifications.sql` ajoute les qualifications OpenRouter par `local_day + news_event_id`.
- `migrations/0013_add_daily_summary_scheduler_state.sql` ajoute le watermark du scheduler de resume.
- `migrations/0014_drop_news_audio_pipeline_candidates.sql` supprime l'ancienne table de benchmark audio candidats.
- `migrations/0015_add_translation_glossary_entries.sql` ajoute le glossaire metier `EN -> FR affiche -> FR vocal`, avec modes `exact`, `word_boundary` et `pattern`; le mode `pattern` supporte les captures `<number>` et `<string>`.
- `migrations/0016_add_daily_summary_admin_runs.sql` ajoute les traces detaillees des runs admin du resume journalier.
- `migrations/0017_add_calendar_day_tables.sql` ajoute les tables canoniques EPIC-032 du calendrier du jour.
- `migrations/0018_add_news_blacklist_and_truncation_audit.sql` ajoute la blacklist operateur appliquee avant persistence/traduction/audio, l'audit des actions blacklist et l'audit des contenus FinancialJuice deja tronques upstream.
- `migrations/0019_add_calendar_day_n8n_metadata.sql` ajoute les identifiants workflow/execution n8n sur les runs calendrier.
- `migrations/0020_drop_legacy_financial_juice_calendar_events.sql` supprime l'ancienne table `calendar_events` du calendrier FinancialJuice non branche.
- `migrations/0021_add_calendar_summary_hls_manifest_paths.sql` ajoute les chemins HLS persistants des resumes calendrier.
- `migrations/0022_seed_translation_glossary_patterns.sql` ajoute le motif idempotent `ECB's <string>` -> `<string> de la BCE`, par exemple `ECB's Nagel` -> `Nagel de la BCE`; la capture `<string>` exclut la ponctuation terminale, ce qui laisse les motifs comme `<string>:` gerer l'espace avant deux-points separement.
- `migrations/0023_add_news_blacklist_target_scope.sql` ajoute `target_scope` aux regles blacklist et `target_scope/action` aux audits pour distinguer rejet complet et retrait de contenu live.
- `migrations/0024_add_retention_and_feed_query_indexes.sql` ajoute les indexes d'expression utilises par les filtres jour, la retention et les lectures feed par source/type.
- `migrations/0025_add_translation_glossary_scope.sql` ajoute la portee `full_pipeline` / `spoken_only` aux regles glossaire.
- `migrations/0026_seed_spoken_time_glossary_patterns.sql` ajoute les motifs vocaux idempotents `14h00 -> quatorze heures` et `12h01 -> douze heures une` en `spoken_only`.
- La retention SQLite active conserve J/J-1 pour `news_events`, `news_translations`, `daily_summaries` et `daily_news_qualifications`.
- Les tables `daily_summary_runs`, `daily_summary_run_steps` et `daily_summary_run_artifacts` sont des traces legacy bornees sur la meme retention J/J-1.
- Les audits append-only `news_blacklist_hits` et `news_truncation_audit` sont bornes par `NEWS_DATA_AUDIT_RETENTION_DAYS`, defaut `7`.
- Les tables de reference et d'etat (`schema_migrations`, `daily_summary_scheduler_state`, `translation_glossary_entries`, `news_blacklist_entries`) ne sont pas purgees par retention.
- Better Auth est la source de verite de l'identite, de la session et du role.
- PostgreSQL `app.profiles` est la source de verite des profils et preferences.
- La table historique `user_profiles` peut rester presente dans une ancienne base SQLite, mais le runtime Rust ne la lit plus.
- Les parametres du collecteur X sont lus/ecrits dans la table SQLite `x_source_settings` par le backend.
- Les migrations avec `ALTER TABLE` doivent etre appliquees dans l'ordre sur une base qui n'a pas deja la colonne cible.
- Le code Rust garde les garde-fous idempotents pour les bases SQLite existantes, notamment via `column_exists`.
- Les tickets de dev sont ranges dans `docs/tickets/`; les EPIC restent dans `docs/epics/`.
