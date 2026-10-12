# NEWS-097 - Auth edge session ticket

## Objectif

Proteger le WebSocket metadata edge `/ws/live` sans faire appeler Supabase ou Postgres par le Durable Object a chaque connexion.

Le backend Rust reste responsable de l'auth:

1. le client authentifie appelle `/api/edge/live-ticket`;
2. le backend verifie la session via le middleware listener;
3. le backend signe un ticket HMAC court;
4. le client ouvre `/ws/live?ticket=...`;
5. le Worker verifie localement signature, expiration et scope.

## Implementation

Backend:

- `EdgeConfig` ajoute:
  - `NEWS_EDGE_TICKET_ENABLED`;
  - `NEWS_EDGE_TICKET_SECRET`;
  - `NEWS_EDGE_TICKET_TTL_SECS`.
- `GET /api/edge/live-ticket` est expose dans les routes listener.
- le ticket est un JWT `HS256` avec:
  - `sub`;
  - `email`;
  - `role`;
  - `scope=live_metadata`;
  - `iat`;
  - `exp`.

Worker:

- `/ws/live` exige maintenant `ticket`;
- le Worker verifie `HS256` avec `FILFINANCE_EDGE_TICKET_SECRET`;
- le Worker refuse:
  - ticket absent;
  - signature invalide;
  - ticket expire;
  - scope different de `live_metadata`.

Client:

- si `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL` est configure:
  - le client appelle `/api/edge/live-ticket`;
  - ajoute le ticket en query string;
  - ouvre le WebSocket edge;
  - tombe en fallback backend si le ticket ou l'edge echoue.

## Variables

Backend local ou serveur:

```env
NEWS_EDGE_TICKET_ENABLED=true
NEWS_EDGE_TICKET_SECRET=replace-with-the-same-32-plus-char-secret-as-worker
NEWS_EDGE_TICKET_TTL_SECS=60
```

Worker Cloudflare:

```bash
npx wrangler secret put FILFINANCE_EDGE_TICKET_SECRET
```

La valeur Worker doit etre identique a `NEWS_EDGE_TICKET_SECRET`.

## Validation

Commandes:

```bash
cd filfinance-service
cargo fmt --check
cargo test edge
cargo check

cd ../filfinance-edge
npm run check

cd ../filfinance-client
npm run typecheck
npm run build:web
```

Critères d'acceptation:

- le ticket expire rapidement;
- un ticket listener suffit au live metadata;
- un ticket invalide ferme/refuse la socket edge;
- aucun secret Supabase prive n'est expose au client ou au Worker.

## Suite

- `NEWS-098`: load test metadata 1000 users.
