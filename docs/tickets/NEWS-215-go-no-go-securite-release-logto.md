# NEWS-215 - Go/no-go securite et release Logto

> **Archive historique — ne pas utiliser comme gate de release.** La cible Logto
> est supersedee par Better Auth + PostgreSQL. Appliquer
> [le runbook Better Auth](../AUTH-RUNBOOK.md) et
> [le runbook GitLab](../GITLAB-RUNBOOK.md).

## Statut

A valider live avec tokens reels.

## Objectif

Valider la release Logto + SQLite avant promotion.

## Acceptance

- Token mauvais issuer refuse.
- Token mauvaise audience refuse.
- Token expire refuse.
- Role JWT ignore.
- Role SQLite listener refuse sur surfaces moderator.
- Role SQLite moderator/admin accepte sur surfaces attendues.
