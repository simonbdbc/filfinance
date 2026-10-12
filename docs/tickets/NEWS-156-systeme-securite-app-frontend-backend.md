# NEWS-156 - Systeme securite App frontend-backend

## Objectif

Centraliser la securite de toutes les routes frontend vers backend Rust autour d'un contrat unique: Supabase JWT verifie cote backend et autorisation par role applicatif Fil Finance.

## Scope

- Classer les routes backend par niveau:
  - `listener`;
  - `moderator`;
  - `admin`.
- Ajouter un garde-fou de boot production: refuser `NEWS_AUTH_ENABLED=false` quand `NEWS_SERVER_ENV=production`.
- Verifier que les routes HTTP frontend transmettent toujours `Authorization: Bearer <access_token>`.
- Verifier que les WebSockets backend exigent un premier message `{ "type": "auth", "access_token": "..." }` avant toute donnee metier.
- Documenter que le role applicatif vient de Supabase/Postgres, pas du JWT Supabase.
- Preparer le passage de `/api/sources` vers admin-only dans `NEWS-157`.

## Hors scope

- Securite Worker Edge.
- Securite HLS.
- Changement du schema des profils utilisateur.
- Migration du hot path news/audio hors SQLite.

## Regles cible

| Niveau | Role minimal | Surfaces |
| --- | --- | --- |
| `listener` | `listener` | feed, historique, replay metadata, ticket edge live, ticket HLS, profil utilisateur |
| `moderator` | `moderator` | test-runs, logs live si conserve moderator, runtime lecture |
| `admin` | `admin` | sources detaillees, switches runtime, health detaille, operations sensibles |

## Criteres d'acceptation

- Une requete sans bearer vers une route app retourne `401`.
- Un bearer valide mais role insuffisant retourne `403`.
- En production (`NEWS_SERVER_ENV=production`), le service refuse de demarrer si `NEWS_AUTH_ENABLED=false`.
- Aucun handler app ne lit un role depuis le JWT Supabase.
- Les WebSockets backend n'emettent ni `hello`, ni snapshot, ni evenement avant auth valide.

## Tests attendus

- Tests unitaires du garde-fou production.
- Tests HTTP:
  - pas de bearer;
  - bearer invalide;
  - bearer valide role insuffisant;
  - bearer valide role attendu.
- Tests WebSocket:
  - fermeture si pas d'auth;
  - fermeture si token invalide;
  - acceptation si token valide.

## Dependances

- `EPIC-026`
- `EPIC-014`
- Supabase Auth et profils Postgres operationnels
