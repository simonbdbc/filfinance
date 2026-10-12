# NEWS-213 - HTTP/WebSocket smokes Logto

> **Archive historique — ne pas executer.** Ces smokes Logto sont supersedes par
> la suite Better Auth actuelle. Utiliser
> [le runbook Better Auth](../AUTH-RUNBOOK.md) et
> [le runbook GitLab](../GITLAB-RUNBOOK.md).

## Statut

Livre cote repo.

## Objectif

Adapter les smokes auth aux tokens Logto.

## Acceptance

- `scripts/smoke_test_logto_auth.sh` utilise `LOGTO_LISTENER_ACCESS_TOKEN`.
- `scripts/smoke_test_websocket_roles.mjs` utilise `LOGTO_LISTENER_ACCESS_TOKEN`.
- Le token moderator reste optionnel.
