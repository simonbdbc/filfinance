# NEWS-157 - Source status heartbeat et API sources admin

## Objectif

Decoupler les badges sources publics de `/api/sources`.

Les badges publics doivent etre alimentes par le canal live, tandis que `/api/sources` devient une route admin directe vers le backend Rust avec details complets et switches.

## Scope

- Ajouter un resume source dans le snapshot `hello`.
- Ajouter un resume source dans le `heartbeat`.
- Garder le payload public compact:
  - `source`;
  - `enabled`;
  - `status`;
  - `updated_at`.
- Ne pas exposer les diagnostics detailles dans le heartbeat.
- Passer `/api/sources` en admin-only.
- Adapter le frontend:
  - `listener`: lit seulement les badges via WebSocket live;
  - `moderator`: lit les badges via WebSocket live;
  - `admin`: lit les badges via WebSocket live et charge `/api/sources` dans l'admin.
- Garder les switches et details runtime dans la page admin.

## Hors scope

- Nouveau design des badges.
- Ajout de nouvelles sources.
- Securite HLS.
- Worker Edge HMAC publish.

## Contrat public

```json
{
  "source": "financial_juice_browser",
  "enabled": true,
  "status": "active",
  "updated_at": "2026-05-09T09:12:00Z"
}
```

Le champ `status` doit rester stable:

- `active`;
- `disabled`;
- `blocked`;
- `degraded`;
- `stale`.

## Contrat admin

`GET /api/sources` retourne les details complets:

- configuration effective;
- etat runtime;
- raison de coupure automatique;
- diagnostic collecteur;
- fallback actif ou idle;
- switches disponibles;
- dernier update.

Cette route exige le role `admin`.

## Criteres d'acceptation

- Un `listener` ne declenche plus d'appel `/api/sources`.
- Un `listener` voit les badges sources via WebSocket live.
- `/api/sources` retourne `403` pour `listener` et `moderator`.
- Un `admin` voit les details sources et les switches dans la page admin.
- Le badge `financial_juice_browser` devient rouge si le collector est bloque, stale ou sans `mainFeed`.
- Aucune information sensible n'est presente dans `hello` ou `heartbeat`.

## Tests attendus

- Tests backend du payload source public.
- Tests backend d'autorisation `/api/sources`.
- Tests frontend de rendu listener/moderator/admin.
- Smoke navigateur sur feed et admin sources.

## Validation locale

- `cargo test`
- `npm run check` dans `filfinance-edge`
- `npm run typecheck` dans `filfinance-client`
- `npm run build:web` dans `filfinance-client`

## Dependances

- `EPIC-026`
- `NEWS-156`
- `EPIC-018`
