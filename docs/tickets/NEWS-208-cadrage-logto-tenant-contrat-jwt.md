# NEWS-208 - Cadrage Logto, tenant et contrat JWT

> **Archive historique — ne pas executer.** Le palier Logto est supersede par
> Better Auth + PostgreSQL. Utiliser [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Statut

Livre cote repo.

## Objectif

Documenter le tenant Logto, l'application native, l'API resource et les variables client/backend.

## Acceptance

- [LOGTO-CONFIG](../LOGTO-CONFIG.md) decrit endpoint, app ID, redirect URI, issuer, JWKS et audience.
- Aucun secret serveur n'est attendu cote client.
- Le contrat HTTP/WebSocket Bearer reste inchange.
