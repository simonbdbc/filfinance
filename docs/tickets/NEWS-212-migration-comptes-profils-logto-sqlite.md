# NEWS-212 - Migration comptes et profils vers Logto + SQLite

> **Archive historique — ne pas executer.** Cette procedure Logto + SQLite est
> supersedee par Better Auth + PostgreSQL. Utiliser
> [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Statut

Procedure documentee.

## Objectif

Encadrer la reprise par invitation/reset sans migration de mots de passe.

## Acceptance

- La procedure est decrite dans [LOGTO-CONFIG](../LOGTO-CONFIG.md).
- Les roles existants doivent etre reappliques dans SQLite.
- Le rollback pointe vers la branche backup creee avant EPIC-036.
