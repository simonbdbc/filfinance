# Audit SQL et retention data

Date d'audit: 2026-06-12.

Sources inspectees:

- Schema consolide: `docs/sql/schema.sql`
- Runtime SQLite: `filfinance-service/src/features/news/sqlite/**`
- Retention applicative: `filfinance-service/src/features/news/sqlite/retention.rs`
- Base locale en lecture seule: `volumes/service/data/news.db`

Les tables internes n8n/Postgres ne font pas partie de ce premier audit. Elles doivent etre traitees dans un audit separe si l'exploitation veut borner l'historique n8n.

## Synthese

La base applicative principale est une SQLite locale append-heavy pour le flux news/audio. La retention existante bornait deja les news, traductions, resumes journaliers et tables `calendar_day_*`, mais elle ne bornait pas les traces legacy `daily_summary_run_*` ni les audits append-only `news_blacklist_hits` et `news_truncation_audit`.

La correction appliquee ajoute:

- retention J/J-1 sur `daily_summary_runs`, `daily_summary_run_steps`, `daily_summary_run_artifacts`;
- retention configurable sur les audits via `NEWS_DATA_AUDIT_RETENTION_DAYS`, defaut `7`;
- indexes d'expression pour les filtres jour et la retention;
- indexes composites pour eviter le tri temporaire sur les lectures feed par `source` et `event_kind`.

## Volumetrie locale observee

Lecture locale: `sqlite3 'file:volumes/service/data/news.db?mode=ro&immutable=1'`.

| Table | Lignes | Min date | Max date | Statut |
| --- | ---: | --- | --- | --- |
| `news_events` | 483 | 2026-06-12T12:56:10Z | 2026-06-12T14:21:45Z | nettoyee J/J-1 |
| `news_translations` | 147 | 2026-06-12T12:56:20Z | 2026-06-12T14:21:47Z | nettoyee J/J-1 |
| `daily_summaries` | 4 | 2026-06-12T12:56:08Z | 2026-06-12T13:02:09Z | nettoyee J/J-1 |
| `daily_news_qualifications` | 0 | n/a | n/a | nettoyee J/J-1 |
| `daily_summary_runs` | 18 | 2026-06-04T22:01:04Z | 2026-06-12T13:02:09Z | maintenant bornee J/J-1 |
| `daily_summary_run_steps` | 102 | 2026-06-04T22:01:04Z | 2026-06-12T13:02:09Z | maintenant bornee J/J-1 |
| `daily_summary_run_artifacts` | 30 | 2026-06-04T22:01:04Z | 2026-06-12T13:02:09Z | maintenant bornee J/J-1 |
| `calendar_day_state` | 1 | 2026-06-12T13:16:45Z | 2026-06-12T13:16:45Z | nettoyee semaine ouvree active |
| `calendar_day_*` enfants | 0 | n/a | n/a | nettoyes semaine ouvree active |
| `news_blacklist_hits` | 0 | n/a | n/a | maintenant bornee, 7 jours par defaut |
| `news_truncation_audit` | 0 | n/a | n/a | maintenant bornee, 7 jours par defaut |
| `translation_glossary_entries` | 2 | 2026-06-03T16:41:59Z | 2026-06-12T13:11:35Z | permanente |
| `news_blacklist_entries` | 0 | n/a | n/a | permanente |
| `schema_migrations` | 19 | 2026-05-31 20:50:36 | 2026-06-12 13:11:35 | permanente |
| `daily_summary_scheduler_state` | 1 | 2026-06-12T13:02:09Z | 2026-06-12T13:02:09Z | permanente |

Note: la base locale inspectee ne contient pas encore `user_profiles`; la table est creee idempotemment au demarrage quand le profil store SQLite est initialise.

## Classification des tables

| Famille | Tables | Grain | Politique |
| --- | --- | --- | --- |
| News brutes | `news_events` | 1 evenement source canonique par `signature` | J/J-1 par date produit |
| Traductions | `news_translations` | 1 traduction par `news_event_id + language` | J/J-1, avec purge des orphelins |
| Resume journalier legacy | `daily_summaries`, `daily_news_qualifications` | 1 resume par `local_day + content_hash`, 1 qualification par `local_day + news_event_id` | J/J-1 |
| Runs legacy | `daily_summary_runs`, `daily_summary_run_steps`, `daily_summary_run_artifacts` | 1 run par `run_id`, enfants par `run_id` | J/J-1, enfants avant parents |
| Calendrier economique | `calendar_day_state`, `calendar_day_runs`, `calendar_day_run_steps`, `calendar_day_run_artifacts`, `calendar_day_items`, `calendar_day_item_sources`, `calendar_day_item_audio`, `calendar_day_summaries`, `calendar_day_alerts` | `local_day`, puis items/runs/audio par IDs dedies | semaine ouvree active, semaine suivante conservee le week-end |
| Audits append-only | `news_blacklist_hits`, `news_truncation_audit` | 1 audit par action ou signature/champ/marqueur | `NEWS_DATA_AUDIT_RETENTION_DAYS`, defaut `7` |
| References | `translation_glossary_entries`, `news_blacklist_entries` | regles metier admin | permanente |
| Etats singleton | `schema_migrations`, `daily_summary_scheduler_state` | etat technique | permanente |
| Profils historiques | `user_profiles` SQLite | ancien palier, non lu par le runtime | hors retention jusqu'a suppression explicite |
| Reglages persistants | `x_source_settings`, `translation_glossary_entries` | singleton/source ou glossaire | permanente |

## Requetes critiques

Observations avant optimisation:

- `list_news` avec filtre `source` utilisait `idx_news_events_source_id_desc` puis une B-tree temporaire pour `ORDER BY COALESCE(published_at, ingested_at) DESC, id DESC`.
- Les filtres jour et la retention avec `substr(COALESCE(...), 1, 10)` pouvaient scanner les indexes existants ou la table.
- L'historique des traductions et les lectures par `language + status` etaient deja correctement couverts par `idx_news_translations_language_status_published_at_desc`.

Indexes ajoutes:

- `idx_news_events_source_published_at_desc`
- `idx_news_events_event_kind_published_at_desc`
- `idx_news_events_visible_day`
- `idx_news_events_retention_day`
- `idx_news_translations_retention_day`

Ces indexes sont declares dans le bootstrap Rust et dans `docs/sql/migrations/0024_add_retention_and_feed_query_indexes.sql`.

## Controles d'exploitation

Volumetrie par table:

```sql
SELECT 'news_events', COUNT(*), MIN(ingested_at), MAX(ingested_at) FROM news_events
UNION ALL SELECT 'news_translations', COUNT(*), MIN(created_at), MAX(created_at) FROM news_translations
UNION ALL SELECT 'daily_summary_runs', COUNT(*), MIN(created_at), MAX(created_at) FROM daily_summary_runs
UNION ALL SELECT 'daily_summary_run_steps', COUNT(*), MIN(created_at), MAX(created_at) FROM daily_summary_run_steps
UNION ALL SELECT 'daily_summary_run_artifacts', COUNT(*), MIN(created_at), MAX(created_at) FROM daily_summary_run_artifacts
UNION ALL SELECT 'news_blacklist_hits', COUNT(*), MIN(created_at), MAX(created_at) FROM news_blacklist_hits
UNION ALL SELECT 'news_truncation_audit', COUNT(*), MIN(created_at), MAX(created_at) FROM news_truncation_audit;
```

Plans a surveiller:

```sql
EXPLAIN QUERY PLAN
SELECT id
FROM news_events
WHERE source = 'financial_juice'
ORDER BY COALESCE(published_at, ingested_at) DESC, id DESC
LIMIT 101;

EXPLAIN QUERY PLAN
SELECT id
FROM news_events
WHERE substr(COALESCE(published_at, ingested_at), 1, 10) IN ('2026-06-12')
ORDER BY COALESCE(published_at, ingested_at) ASC, id ASC
LIMIT 2000;

EXPLAIN QUERY PLAN
DELETE FROM news_events
WHERE substr(COALESCE(published_at, event_time, ingested_at), 1, 10) < '2026-06-11';
```

## Risques residuels

- Les seuils sont calendaires, pas volumetriques. Si le volume intraday augmente fortement, ajouter une alerte taille fichier SQLite et `freelist_count`.
- Les audits blacklist/troncature sont maintenant bornes. Si un besoin legal ou support impose un historique plus long, augmenter `NEWS_DATA_AUDIT_RETENTION_DAYS` plutot que rendre ces tables permanentes.
- Better Auth/PostgreSQL et n8n ont leurs propres stockages. Ils ne sont pas couverts par cette retention SQLite.
