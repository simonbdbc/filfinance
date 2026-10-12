# NEWS-165 - Validation charge cache playlists HLS Worker

## Objectif

Valider que le cache tres court des playlists reduit la latence origin et permet de viser
1000 auditeurs HLS avec une marge exploitable.

## Scope

- Adapter `scripts/load_backend_ws_hls.mjs` si necessaire pour reporter les statuts playlist.
- Tester 500 auditeurs avec TTL playlist actif.
- Tester 1000 auditeurs si le palier 500 est stable.
- Comparer avant/apres:
  - latence playlist;
  - `PLAYLIST_HIT` / `PLAYLIST_MISS`;
  - hit ratio segments;
  - erreurs ticket;
  - CPU/RAM backend;
  - logs Traefik/Rust origin.

## Commandes cible

Commandes historiques: ne pas relancer contre Cloudflare CDN standard depuis `EPIC-034`.
Remplacer la base HLS par une cible media approuvee.

Palier 500:

```bash
FILFINANCE_LOAD_TEST_CONNECT_BATCH_SIZE=10 \
FILFINANCE_LOAD_TEST_CONNECT_BATCH_DELAY_MS=500 \
FILFINANCE_LOAD_TEST_TIERS=500 \
FILFINANCE_LOAD_TEST_DURATION_MS=60000 \
FILFINANCE_LOAD_TEST_SKIP_WS=true \
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://media-dns-only.example.test" \
node scripts/load_backend_ws_hls.mjs
```

Palier 1000:

```bash
FILFINANCE_LOAD_TEST_CONNECT_BATCH_SIZE=10 \
FILFINANCE_LOAD_TEST_CONNECT_BATCH_DELAY_MS=500 \
FILFINANCE_LOAD_TEST_TIERS=1000 \
FILFINANCE_LOAD_TEST_DURATION_MS=60000 \
FILFINANCE_LOAD_TEST_SKIP_WS=true \
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://media-dns-only.example.test" \
node scripts/load_backend_ws_hls.mjs
```

## Criteres d'acceptation 500

- `ok=true`;
- `playlist.failures=0`;
- `segment.failures=0`;
- `ticket.failures=0`;
- `playlist.hls_proxy_cache_status.PLAYLIST_HIT` majoritaire apres warmup;
- `segment.hls_proxy_cache_status.HIT` majoritaire;
- p95 playlist nettement inferieur au run sans cache court.

## Criteres d'acceptation 1000

- `ok=true`;
- `playlist.failures=0`;
- `segment.failures=0`;
- `ticket.failures=0`;
- hit ratio segment stable;
- hit ratio playlist stable;
- p95/p99 archives;
- CPU/RAM backend releves en parallele.

## Resultat valide

Run replay HLS Worker du 11 mai 2026, palier 1000 auditeurs, TTL playlist `2s`:

- `ok=true`;
- `ticket.requests=1000`, `ticket.failures=0`, p95 `122.49ms`, p99 `196.45ms`;
- `playlist.requests=40067`, `playlist.failures=0`, p95 `42.61ms`, p99 `90.03ms`;
- `playlist.hls_proxy_cache_status.PLAYLIST_HIT=38974`;
- `playlist.hls_proxy_cache_status.PLAYLIST_MISS=1093`;
- hit ratio playlist environ `97.3%`;
- `segment.requests=40067`, `segment.failures=0`, p95 `34.24ms`, p99 `47.23ms`;
- `segment.hls_proxy_cache_status.HIT=40064`;
- `segment.hls_proxy_cache_status.MISS=3`;
- bytes transferes: `1288404984`.

Decision: le volet HLS replay via Worker est valide a 1000 auditeurs dans ces conditions
de test. Le test ne valide pas le WebSocket metadata live, car `FILFINANCE_LOAD_TEST_SKIP_WS`
etait actif.

## Sortie attendue

Ajouter au compte rendu de charge:

- date et heure du run;
- commit backend/Worker;
- TTL playlist;
- paliers testes;
- resume JSON cle;
- interpretation et decision go/no-go.

## Dependances

- `EPIC-027`
- `NEWS-164`
- `NEWS-162`
