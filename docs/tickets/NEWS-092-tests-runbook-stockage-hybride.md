# NEWS-092 - Tests et runbook du stockage hybride

> **Archive historique — ne pas executer.** Ces smokes Supabase et ce stockage
> hybride sont supersedes par Better Auth + PostgreSQL. Utiliser
> [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Objectif

Verrouiller l'exploitation du modele hybride: Supabase Auth plus Supabase Postgres pour profils/roles/preferences, SQLite local pour news/audio.

## Statut

Implemente cote repo le 2 mai 2026.

Les smokes sont prets pour validation live avec tokens Supabase:

- `filfinance-service/scripts/smoke_test_supabase_auth.sh`
- `filfinance-service/scripts/smoke_test_websocket_roles.mjs`

## Scope

- smoke test HTTP avec profil Postgres
- validation WebSocket listener/moderator
- diagnostic des pannes Postgres profil
- diagnostic des pannes SQLite pipeline
- documentation de promotion `moderator`
- checklist de rollback si un fallback local est conserve

## Hors scope

- monitoring production complet
- dashboard admin
- migration des news vers Postgres

## Approche technique

Mettre a jour les runbooks existants au lieu de dupliquer la documentation auth.

Le diagnostic doit separer trois familles d'incident:

- Supabase Auth: token invalide, JWKS, expiration, issuer/audience;
- Supabase Postgres profil: connexion, droits serveur, table absente, role invalide;
- SQLite pipeline: migrations news/audio, fichier non writable, disque plein.

Commandes de validation:

```bash
cd filfinance-service
FILFINANCE_API_BASE_URL="https://filfinance-api.sbcenter.fr" \
SUPABASE_LISTENER_ACCESS_TOKEN="${SUPABASE_LISTENER_ACCESS_TOKEN}" \
SUPABASE_MODERATOR_ACCESS_TOKEN="${SUPABASE_MODERATOR_ACCESS_TOKEN:-}" \
bash scripts/smoke_test_supabase_auth.sh
```

```bash
cd filfinance-service
FILFINANCE_API_BASE_URL="https://filfinance-api.sbcenter.fr" \
SUPABASE_LISTENER_ACCESS_TOKEN="${SUPABASE_LISTENER_ACCESS_TOKEN}" \
SUPABASE_MODERATOR_ACCESS_TOKEN="${SUPABASE_MODERATOR_ACCESS_TOKEN:-}" \
node scripts/smoke_test_websocket_roles.mjs
```

## Criteres d'acceptation

- le smoke test confirme `GET /api/me` avec profil Postgres
- le smoke test confirme `PATCH /api/me` avec `preferred_tts_voice` et `theme` Postgres
- le smoke test confirme qu'un `theme` inconnu est refuse en `400`
- un listener reste refuse sur runtime et `/ws/test-runs`
- un moderator est accepte sur runtime et `/ws/test-runs`
- le runbook indique comment promouvoir un utilisateur en `moderator`
- la doc explique que les news/audio restent en SQLite

## Tests attendus

- `bash -n filfinance-service/scripts/smoke_test_supabase_auth.sh`: OK
- `node --check filfinance-service/scripts/smoke_test_websocket_roles.mjs`: OK
- smoke HTTP avec token listener: script pret, preferences `preferred_tts_voice` et `theme` incluses
- smoke HTTP avec token moderator: script pret si `SUPABASE_MODERATOR_ACCESS_TOKEN` est fourni
- validation WebSocket listener/moderator: script pret
- test manuel de panne Postgres profil documente: OK dans `AUTH-RUNBOOK`
- test manuel de panne SQLite pipeline documente: OK dans `AUTH-RUNBOOK`

## Dependances

- [NEWS-090 - Bascule auth HTTP/WebSocket vers profils Postgres](./NEWS-090-bascule-auth-profils-postgres.md)
- [NEWS-091 - Backfill profils SQLite vers Supabase Postgres](./NEWS-091-backfill-profils-sqlite-vers-supabase-postgres.md)
- [AUTH-RUNBOOK](../AUTH-RUNBOOK.md)
