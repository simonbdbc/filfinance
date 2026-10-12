# Fil Finance Edge

Worker Cloudflare de metadata live pour `EPIC-018`.

## Endpoints

- `GET /health`: verification publique du Worker.
- `GET /ws/live?ticket=...`: WebSocket de metadata live, porte par le Durable Object `filfinance-live`.
- `POST /publish/live`: endpoint serveur de publication metadata. Il exige une signature HMAC avec `FILFINANCE_EDGE_PUBLISH_SECRET`.
- `GET /hls/audio/...`: proxy HLS vers l'origin Rust. Le Worker transmet `hls_ticket` a l'origin, ne valide pas le ticket lui-meme, met en cache les segments `.ts` et peut mettre les playlists en cache tres court.

## Variables

- `LIVE_OBJECT_NAME`: nom du Durable Object, par defaut `filfinance-live`.
- `FILFINANCE_HLS_PROXY_ENABLED`: active explicitement le proxy HLS si la valeur vaut
  `true`. Toute autre valeur ferme `/hls/audio/...` avec `410 Gone`.
- `FILFINANCE_HLS_ORIGIN_BASE_URL`: base HTTPS de l'origin HLS Rust, par defaut `https://api.filfinance.fr`.
- `FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS`: TTL tres court des playlists HLS. `0` ou vide desactive; `1` ou `2` active; le code limite a `5`.

## Secrets

Le secret de publication ne doit pas etre versionne:

```bash
npx wrangler secret put FILFINANCE_EDGE_PUBLISH_SECRET
npx wrangler secret put FILFINANCE_EDGE_TICKET_SECRET
```

`FILFINANCE_EDGE_TICKET_SECRET` doit avoir la meme valeur que `NEWS_EDGE_TICKET_SECRET` cote backend.
`FILFINANCE_EDGE_PUBLISH_SECRET` doit avoir la meme valeur que `NEWS_EDGE_PUBLISH_SECRET` cote backend.
Les deux secrets doivent contenir au moins 32 caracteres.

Le Worker et son Durable Object sont réservés à la production Cloudflare. Le pilote DEV Bunny
utilise le WebSocket Rust/Redis et ne déploie ni Worker, ni secrets Cloudflare.

## Publication backend

Le backend signe chaque body brut avec une horloge courte:

```text
X-FilFinance-Timestamp: 2026-05-09T09:12:00.000Z
X-FilFinance-Signature: sha256=<base64url-hmac>
```

La signature est:

```text
HMAC_SHA256(secret, timestamp + "." + raw_body)
```

Le Worker refuse une signature absente, invalide ou hors fenetre de validite.

## Proxy HLS

Le proxy HLS est ferme par defaut depuis `EPIC-034`, apres mitigation Cloudflare Trust &
Safety sur la livraison media. `live.filfinance.fr` reste le control plane Edge
metadata. Ne reactiver `FILFINANCE_HLS_PROXY_ENABLED=true` qu'apres validation d'une
cible media conforme.

Le proxy HLS expose les chemins suivants:

```text
/hls/audio/replay/today/playlist.m3u8
/hls/audio/replay/today/segments/{local_day}/{segment_name}
/hls/audio/clips/{stream_id}/playlist.m3u8
/hls/audio/clips/{stream_id}/{segment_name}
```

Le backend Rust reste responsable de la securite HLS: le client demande
`GET /api/hls-ticket`, puis ajoute `hls_ticket` a la playlist. Le Worker transmet
la query string a l'origin.

Les playlists renvoyees par l'origin sont reecrites par le Worker: les lignes de segments
`/audio/...` deviennent `/hls/audio/...`, afin que le player repasse par le proxy pour
les segments.

Le header `X-FilFinance-HLS-Proxy-Cache` permet de verifier le comportement:

- `HIT`: segment servi depuis le cache Worker;
- `MISS`: segment recupere depuis l'origin puis stocke;
- `PLAYLIST_HIT`: playlist servie depuis le cache Worker;
- `PLAYLIST_MISS`: playlist recuperee depuis l'origin, reecrite puis stockee;
- `BYPASS_PLAYLIST_DISABLED`: cache playlist desactive;
- `BYPASS_ORIGIN_ERROR`: reponse origin non `200`, non stockee;
- `BYPASS_RANGE` ou `BYPASS_METHOD`: requete non cacheable.

## Validation locale

```bash
npm run check
```

## Load test metadata

Le script ouvre des clients WebSocket, publie des messages metadata et mesure les livraisons:

```bash
FILFINANCE_EDGE_WS_URL="wss://live.filfinance.fr/ws/live" \
FILFINANCE_EDGE_LOAD_TEST_TICKET="<edge-live-ticket>" \
FILFINANCE_EDGE_PUBLISH_URL="https://live.filfinance.fr/publish/live" \
FILFINANCE_EDGE_PUBLISH_SECRET="<publish-secret>" \
FILFINANCE_EDGE_LOAD_TEST_TIERS=100,500,1000 \
npm run load-test:live
```

Sans `FILFINANCE_EDGE_LOAD_TEST_TIERS`, le script execute un seul palier avec
`FILFINANCE_EDGE_LOAD_TEST_CLIENTS`.

Le rapport JSON contient:

- clients connectes, echecs, fermetures inattendues;
- types de messages publies et recus;
- livraisons attendues/recues/manquantes;
- latences p50/p95/p99/max;
- verification du snapshot `hello`;
- verification des sources dans le `heartbeat`.

Les messages publies couvrent `translated_news_item`, `audio_ready`, `audio_failed`,
`audio_finalized`, `system_notice` et `source_status`.

## Load test HLS

Le proxy HLS se teste depuis la racine du repo applicatif avec
`scripts/load_backend_ws_hls.mjs`, uniquement sur une cible media non Cloudflare ou
explicitement approuvee. Les bases `filfinance.fr`, `workers.dev`,
`cloudflarestream.com` et `videodelivery.net` sont bloquees par defaut.

Exemple replay HLS seul vers une cible DNS-only approuvee:

```bash
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://media-dns-only.example.test" \
FILFINANCE_LOAD_TEST_TIERS=1000 \
FILFINANCE_LOAD_TEST_DURATION_MS=60000 \
FILFINANCE_LOAD_TEST_SKIP_WS=true \
node scripts/load_backend_ws_hls.mjs
```

Run historique du 11 mai 2026 a 1000 auditeurs HLS replay, a ne pas relancer via
Cloudflare CDN standard:

- `ok=true`;
- `playlist.failures=0`, p95 `42.61ms`, p99 `90.03ms`;
- `segment.failures=0`, p95 `34.24ms`, p99 `47.23ms`;
- `ticket.failures=0`;
- `PLAYLIST_HIT=38974`, `PLAYLIST_MISS=1093`;
- `HIT=40064`, `MISS=3` pour les segments.

## Deploiement

```bash
npx wrangler deploy
```
