# NEWS-211 - Profils, roles et preferences en SQLite uniquement

> **Archive historique — ne pas executer.** La cible profil SQLite de ce ticket
> est supersedee par Better Auth + PostgreSQL. Utiliser
> [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Statut

Livre cote repo.

## Objectif

Faire de SQLite la seule source runtime pour `user_profiles`, roles, preferences et reglages X.

## Acceptance

- `NEWS_PROFILE_STORE=sqlite` est la seule cible supportee.
- `auth_user_id` remplace l'ancien nom fournisseur-specifique.
- Les reglages X sont persistés en SQLite.
- Le backend ne charge plus de pool Postgres profil.
