# NEWS-089 - Repository backend hybride des profils

## Objectif

Introduire une couche de persistence profils distincte du stockage SQLite news/audio, afin de brancher Supabase Postgres pour les profils, roles et preferences utilisateur.

## Statut

Implemente cote repo le 2 mai 2026.

Le store Supabase Postgres est actif quand `NEWS_PROFILE_STORE=supabase_postgres` et `NEWS_SUPABASE_DATABASE_URL` est configure. Le fallback `sqlite` reste disponible pour les tests et le developpement local.

## Scope

- configuration serveur du store profil
- facade ou trait de repository profil
- implementation Supabase Postgres cible
- fallback SQLite explicite si conserve pour le dev local
- separation nette entre stockage profil et `NewsStorage`
- erreurs lisibles quand le store profil est indisponible

## Hors scope

- migration des tables news/audio vers Postgres
- changement du contrat HTTP client
- backfill des profils existants
- UI admin de gestion des roles

## Approche technique

Ajouter une abstraction dediee, par exemple `UserProfileStore`, avec les operations deja necessaires:

- `upsert_user_profile_from_auth`
- `get_user_profile`
- `update_user_preferences`

Le pipeline news/audio doit continuer a utiliser SQLite via `NewsStorage`. Le store profil peut etre choisi par configuration:

```env
NEWS_PROFILE_STORE=supabase_postgres
NEWS_SUPABASE_DATABASE_URL=postgresql://postgres.ycgerpksnqlbvulmkmkh:database-password@aws-0-eu-west-1.pooler.supabase.com:5432/postgres?sslmode=require
NEWS_PROFILE_STORE_MAX_CONNECTIONS=5
```

Un mode local transitoire peut rester disponible pour permettre les tests sans Supabase Postgres, mais il doit etre nomme explicitement et documente comme fallback.

## Criteres d'acceptation

- `NewsStorage` ne porte plus la responsabilite long terme des profils utilisateur
- le backend compile avec le store profil selectionne par configuration
- le store Postgres cree un profil `listener` pour un utilisateur Supabase inconnu
- l'upsert d'identite ne remplace pas un role existant
- l'upsert d'identite ne supprime pas `preferred_tts_voice`
- les erreurs de connexion ou de requete profil sont remontees sans masquer une panne d'auth

## Tests attendus

- tests unitaires du choix de configuration: OK
- tests repository avec SQLite fallback si conserve: OK
- tests repository Postgres si l'environnement de CI/dev fournit `NEWS_SUPABASE_DATABASE_URL`
- `cargo fmt --check`: OK
- `cargo check`: OK
- `cargo test`: OK

## Dependances

- [EPIC-017 - Stockage hybride profils Supabase Postgres](../epics/EPIC-017-stockage-hybride-profils-supabase-postgres.md)
- [NEWS-088 - Schema Supabase Postgres des profils](./NEWS-088-stockage-hybride-profils-supabase-postgres.md)
- [SQL Supabase Postgres](../sql/supabase/README.md)
