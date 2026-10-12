# NEWS-094 - Worker et Durable Object live

## Statut

Implementation livree le 3 mai 2026.

## Objectif

Creer le premier Worker Cloudflare de metadata live et le Durable Object charge du fan-out WebSocket.

Le Worker ne remplace pas encore le backend Rust. Il prepare le control plane edge qui sera branche par `NEWS-095`.

## Scope

- Package `filfinance-edge` dedie au Worker.
- Route publique `GET /health`.
- Route WebSocket publique `GET /ws/live`.
- Route serveur `POST /publish/live`.
- Durable Object `LiveMetadataDurableObject`.
- Fan-out d'un message publie vers tous les WebSockets connectes.
- Snapshot minimal pour reconnexion:
  - `latest_event_id`;
  - `now_playing`;
  - `audio_state`.
- Secret de publication `FILFINANCE_EDGE_PUBLISH_SECRET`.
- Tests Node du protocole de publication.

## Hors scope

- Deploiement Cloudflare live.
- Branchement du publisher Rust.
- Ticket d'auth edge court pour les clients.
- Load test 1000 users.
- Audio HLS.

## Implementation

- `filfinance-edge/wrangler.jsonc`:
  - Worker `filfinance-live-edge`;
  - binding Durable Object `LIVE_METADATA`;
  - migration SQLite-backed `LiveMetadataDurableObject`.
- `filfinance-edge/src/index.js`:
  - routing Worker;
  - verification du secret de publication;
  - Durable Object hibernatable via `ctx.acceptWebSocket`;
  - hello initial avec snapshot;
  - persistence minimale du snapshot dans le storage du Durable Object.
- `filfinance-edge/src/protocol.js`:
  - validation des messages publishables;
  - refus des payloads ressemblant a des chunks audio;
  - helpers `hello`, `heartbeat`, snapshot.

## Contrat runtime

Connexion client:

```text
GET /ws/live
Upgrade: websocket
```

Publication backend:

```text
POST /publish/live
X-FilFinance-Timestamp: 2026-05-09T09:12:00Z
X-FilFinance-Signature: sha256=<base64url-hmac>
Content-Type: application/json
```

Depuis `NEWS-158`, la publication serveur vers Worker est signee avec `HMAC_SHA256(secret, timestamp + "." + raw_body)`.

Le corps doit etre un message `NEWS-093` publishable:

- `news_card`;
- `translated_news_item`;
- `now_playing`;
- `audio_state`;
- `system_notice`.

Le Worker refuse les types techniques `hello`, `heartbeat`, `lagged` sur `POST /publish/live`, car ils sont produits par l'edge.

## Criteres d'acceptation

- Plusieurs clients WebSocket peuvent etre attaches au meme Durable Object nomme `filfinance-live`.
- Un message publie une seule fois sur `POST /publish/live` est diffuse a tous les clients connectes.
- Une reconnexion recoit un `hello` avec snapshot minimal.
- Le Durable Object ne stocke pas d'historique volumineux.
- Aucun chunk audio n'est accepte par le validateur de publication.

## Validation locale

- `npm install --package-lock-only --ignore-scripts`: OK.
- `npm run check` dans `filfinance-edge`: OK.

## References

- [Cloudflare Durable Objects WebSocket Hibernation](https://developers.cloudflare.com/durable-objects/examples/websocket-hibernation-server/)
- [Cloudflare Durable Objects migrations](https://developers.cloudflare.com/durable-objects/reference/durable-objects-migrations/)
