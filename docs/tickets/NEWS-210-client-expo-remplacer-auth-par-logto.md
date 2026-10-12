# NEWS-210 - Client Expo: remplacer l'auth par Logto

> **Archive historique — ne pas executer.** Le palier Logto est supersede par
> Better Auth + PostgreSQL. Utiliser [le runbook Better Auth](../AUTH-RUNBOOK.md).

## Statut

Livre cote repo.

## Objectif

Brancher `@logto/rn` dans le client Expo et conserver le contrat `accessToken` existant.

## Acceptance

- Le client utilise `LogtoProvider`.
- Le login passe par redirect OIDC.
- `getAccessToken(EXPO_PUBLIC_LOGTO_API_RESOURCE)` alimente HTTP et WebSocket.
- `npm run typecheck` passe.
