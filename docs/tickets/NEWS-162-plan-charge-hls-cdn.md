# NEWS-162 - Plan charge HLS CDN

## Objectif

Valider que l'audio HLS supporte la charge utilisateur via Cloudflare CDN, sans saturer Rust origin.

## Scope

- Tester les clips HLS progressifs.
- Tester le replay HLS du jour.
- Mesurer les requetes playlist et segment.
- Mesurer le cache HLS Worker avec `X-FilFinance-HLS-Proxy-Cache`.
- Conserver `CF-Cache-Status` dans les rapports pour la future etape domaine proxie Cloudflare.
- Mesurer les requetes qui arrivent vraiment au backend Rust.
- Simuler plusieurs paliers:
  - 50 auditeurs;
  - 100 auditeurs;
  - 200 auditeurs;
  - 500 auditeurs;
  - 1000 auditeurs si le cache CDN est valide.

## Hors scope

- Fan-out metadata Durable Objects.
- Benchmark TTS.
- Benchmark ingestion.

## Scenarios

### Clip HLS

1. Produire un clip HLS reel ou fixture.
2. Publier `audio_ready`.
3. Simuler des clients qui chargent playlist + segments.
4. Verifier que les segments deviennent `HIT` cote Cloudflare.

### Replay du jour

1. Alimenter un replay HLS avec plusieurs segments.
2. Simuler des clients qui chargent la playlist replay puis les derniers segments.
3. Chaque client authentifie recupere `GET /api/hls-ticket` puis charge la playlist avec `hls_ticket`.
4. Mesurer la pression sur la playlist.
5. Verifier que les segments sont majoritairement servis depuis le cache Worker.

## Mesures

- clients demandes;
- duree du test;
- requetes playlist;
- requetes segment;
- erreurs HTTP;
- latence p50/p95/p99/max;
- bytes transferes;
- `X-FilFinance-HLS-Proxy-Cache` par type de ressource;
- `CF-Cache-Status` si disponible;
- hits origin Traefik/Rust;
- requetes `GET /api/hls-ticket`;
- CPU/RAM backend;
- interruptions audio cote client si test navigateur.

## Criteres d'acceptation

- 500 auditeurs HLS sans erreur HTTP significative.
- Objectif final: 1000 auditeurs HLS sans saturation Rust origin.
- Les segments chauds sont majoritairement `HIT` cote Worker HLS proxy.
- Les playlists restent fraiches sans request storm.
- La queue audio reste evenementielle: aucun player HLS actif en idle.
- Les URLs HLS utilisees par le client passent par le systeme de securite HLS.

## Etat valide

Le run 500 auditeurs valide le cache segment Worker.

Le run 1000 auditeurs HLS replay du 11 mai 2026 valide aussi le cache tres court des
playlists Worker, avec `hls_ticket` conserve dans la cache key:

- `ok=true`;
- `playlist.failures=0`;
- `segment.failures=0`;
- `ticket.failures=0`;
- p95 playlist `42.61ms`;
- p95 segment `34.24ms`;
- hit ratio playlist environ `97.3%`;
- hit ratio segment quasi total.

La suite de charge doit valider le scenario combine `NEWS-167`: metadata Durable Objects
et HLS, puis un smoke navigateur pour confirmer le comportement player et les interruptions audio.

## Dependances

- `EPIC-026`
- `EPIC-021`
- `NEWS-159`
- `NEWS-167`
- Worker HLS proxy deploye

## Commande cible

Note `EPIC-034`: ces commandes ne doivent plus cibler Cloudflare CDN standard,
`workers.dev` ou `filfinance.fr` sans opt-in explicite et validation d'une cible media
conforme.

Replay HLS seul:

```bash
FILFINANCE_LOAD_TEST_EMAIL="<listener-email>" \
FILFINANCE_LOAD_TEST_PASSWORD="<password>" \
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://media-dns-only.example.test" \
FILFINANCE_LOAD_TEST_TIERS=50,100,200,500 \
FILFINANCE_LOAD_TEST_DURATION_MS=60000 \
FILFINANCE_LOAD_TEST_SKIP_WS=true \
node scripts/load_backend_ws_hls.mjs
```

Replay + clip HLS:

```bash
FILFINANCE_LOAD_TEST_EMAIL="<listener-email>" \
FILFINANCE_LOAD_TEST_PASSWORD="<password>" \
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://media-dns-only.example.test" \
FILFINANCE_LOAD_TEST_TIERS=50,100,200,500 \
FILFINANCE_LOAD_TEST_DURATION_MS=60000 \
FILFINANCE_LOAD_TEST_SKIP_WS=true \
FILFINANCE_LOAD_TEST_HLS_CLIP_PLAYLIST_URL="/audio/clips/<stream_id>/playlist.m3u8" \
node scripts/load_backend_ws_hls.mjs
```

Palier 1000 HLS replay sur cible media approuvee:

```bash
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://media-dns-only.example.test" \
FILFINANCE_LOAD_TEST_TIERS=1000 \
FILFINANCE_LOAD_TEST_DURATION_MS=60000 \
FILFINANCE_LOAD_TEST_SKIP_WS=true \
node scripts/load_backend_ws_hls.mjs
```

## Implementation

`scripts/load_backend_ws_hls.mjs` mesure maintenant:

- paliers par defaut `50,100,200,500`;
- tickets HLS via `GET /api/hls-ticket`;
- replay HLS du jour;
- clip HLS optionnel via `FILFINANCE_LOAD_TEST_HLS_CLIP_PLAYLIST_URL`;
- demarrage progressif des boucles HLS par batch, pilote par `FILFINANCE_LOAD_TEST_CONNECT_BATCH_SIZE` et `FILFINANCE_LOAD_TEST_CONNECT_BATCH_DELAY_MS`;
- skip explicite des playlists quand aucun ticket HLS n'a pu etre obtenu, via `skipped_no_ticket`;
- requetes playlist et segments;
- latences p50/p95/p99/max;
- bytes transferes;
- `CF-Cache-Status` par type de ressource et par cible `replay` / `clip`;
- `X-FilFinance-HLS-Proxy-Cache` par type de ressource et par cible `replay` / `clip`.

`FILFINANCE_LOAD_TEST_HLS_BASE_URL` permet de tester une cible media separee sans changer
l'URL backend utilisee pour `/api/hls-ticket`. Depuis `EPIC-034`, les cibles Cloudflare
sont bloquees par defaut.

Les hits origin Traefik/Rust, CPU/RAM backend et interruptions audio restent a relever en parallele
dans l'infra serveur ou avec un smoke navigateur, car ils ne sont pas observables depuis un fetch
Node distant.

Validation locale:

```bash
node --check scripts/load_backend_ws_hls.mjs
```
