# NEWS-091 - Backfill profils SQLite vers Supabase Postgres

> **Archive historique — ne pas executer ce backfill.** Cette migration Supabase
> est supersedee par Better Auth + PostgreSQL. Utiliser
> [le guide de migration actuel](../MIGRATION-BETTER-AUTH-POSTGRES.md).

## Objectif

Migrer les profils existants du store SQLite transitoire vers `public.user_profiles` dans Supabase Postgres sans perdre roles ni preferences.

## Statut

Implemente cote repo le 2 mai 2026.

Valide live le 2 mai 2026 sur Supabase Session pooler:

- `total_read=1`
- `imported=1`
- `updated=0`
- `ignored=0`
- `errors=0`

Le backfill est expose comme commande one-shot du backend:

```bash
filfinance-service backfill-user-profiles
```

La commande lit `NEWS_DATABASE_URL` pour les profils SQLite existants et utilise `NEWS_SUPABASE_DATABASE_URL` pour ecrire dans `public.user_profiles`.

## Scope

- lecture des profils SQLite existants
- ecriture idempotente dans Supabase Postgres
- preservation de `supabase_user_id`
- preservation de `email`
- preservation de `role`
- preservation de `preferred_tts_voice`
- rapport des lignes importees, ignorees et en erreur

## Hors scope

- migration des news, traductions ou calendrier
- transformation de roles hors `listener`, `moderator`, `admin` sans decision explicite
- suppression automatique de la table SQLite `user_profiles`

## Approche technique

Le backfill doit etre relancable. La cle d'idempotence est `supabase_user_id`.

Strategie recommandee:

- lire tous les profils SQLite via `NewsStorage::list_user_profiles`;
- valider `role` avant insertion;
- inserer les profils absents dans Postgres;
- mettre a jour `email` et `preferred_tts_voice` si le profil existe deja;
- ne pas degrader un role Postgres existant sans option explicite;
- retourner un rapport JSON avec `total_read`, `imported`, `already_present`, `updated`, `ignored` et `errors`.

Commande locale:

```bash
cd filfinance-service
NEWS_SUPABASE_DATABASE_URL='postgresql://...' cargo run -- backfill-user-profiles
```

Commande serveur Docker Compose:

```bash
cd /home/traefik/filfinance-docker
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml run --rm filfinance-service filfinance-service backfill-user-profiles
```

Le backfill ne supprime pas la table SQLite `user_profiles`.

## Criteres d'acceptation

- relancer le backfill ne duplique aucun profil
- un profil SQLite `moderator` reste `moderator` dans Postgres
- un profil SQLite `listener` ne remplace pas un role Postgres deja `moderator`
- les preferences TTS sont conservees
- les profils invalides sont reportes clairement
- le rapport de fin distingue importes, deja presents, mis a jour, ignores et erreurs

## Tests attendus

- test sur base SQLite fixture avec listener/moderator/admin: OK
- test idempotence double execution: OK via plan de backfill `AlreadyPresent`
- test de role Postgres existant plus privilegie non degrade: OK
- test de preference TTS conservee: OK
- `cargo fmt --check`: OK
- `cargo check`: OK
- `cargo test`: OK

## Dependances

- [NEWS-089 - Repository backend hybride des profils](./NEWS-089-repository-backend-hybride-profils.md)
- [NEWS-090 - Bascule auth HTTP/WebSocket vers profils Postgres](./NEWS-090-bascule-auth-profils-postgres.md)
