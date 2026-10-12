# NEWS-088 - Schema Supabase Postgres des profils

> **Archive historique — ne pas appliquer ce SQL.** Ce schema Supabase est
> supersede par les schemas Better Auth + PostgreSQL documentes dans
> [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Objectif

Formaliser le schema Supabase Postgres qui portera les profils, roles et preferences utilisateur dans l'architecture hybride Fil Finance.

## Statut

Documente le 2 mai 2026. Application dans un vrai projet Supabase a executer pendant le dev de `EPIC-017`.

## Scope

- table `public.user_profiles`
- lien avec `auth.users(id)`
- role par defaut `listener`
- roles autorises `listener`, `moderator`, `admin`
- preference `preferred_tts_voice`
- trigger `updated_at`
- RLS activee sans policy publique dans le premier lot
- documentation du fait que SQLite reste le stockage du pipeline news/audio

## Hors scope

- migration des tables `news_events`, `news_translations` ou `calendar_day_*`
- acces direct client Expo a `public.user_profiles`
- implementation Rust du repository Postgres
- backfill des profils locaux existants

## Approche technique

Ajouter le schema cible dans:

```text
docs/sql/supabase/schema.sql
docs/sql/supabase/migrations/0001_create_user_profiles.sql
```

Le backend Rust continuera de verifier le JWT Supabase avant de lire ou ecrire le profil. La table Postgres devient la source de verite des roles et preferences, mais le client conserve le contrat backend `GET /api/me` et `PATCH /api/me`.

## Criteres d'acceptation

- le schema Postgres peut etre applique dans Supabase SQL Editor
- un profil cree sans role explicite obtient `listener`
- `role` refuse toute valeur hors `listener`, `moderator`, `admin`
- `preferred_tts_voice` reste optionnel et limite a 128 caracteres
- `updated_at` est mis a jour automatiquement lors d'un `UPDATE`
- la doc indique que la `service_role key` reste interdite cote client

## Tests attendus

Validation manuelle dans Supabase:

```sql
INSERT INTO public.user_profiles (supabase_user_id, email)
VALUES ('00000000-0000-0000-0000-000000000000', 'listener@example.test');
```

Dans un vrai projet, remplacer l'UUID par un `auth.users.id` existant a cause de la contrainte de cle etrangere.

Verifier ensuite:

```sql
SELECT supabase_user_id, email, role, preferred_tts_voice
FROM public.user_profiles
WHERE email = 'listener@example.test';
```

## Dependances

- [EPIC-017 - Stockage hybride profils Supabase Postgres](../epics/EPIC-017-stockage-hybride-profils-supabase-postgres.md)
- [SUPABASE-CONFIG](../SUPABASE-CONFIG.md)
- [SQL Supabase Postgres](../sql/supabase/README.md)
