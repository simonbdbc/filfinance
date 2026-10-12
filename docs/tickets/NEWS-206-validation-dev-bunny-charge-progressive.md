# NEWS-206 - Validation dev Bunny et charge progressive

## Objectif

Valider que Bunny peut servir le data plane HLS dev avec une montee en charge progressive,
sans repasser par Cloudflare pour les playlists et segments.

## Scope

- Verifier que `.env.server.dev` active `NEWS_TTS_ENABLED=true` et `NEWS_HLS_ENABLED=true`.
- Valider phase 1 sur `https://<pull-zone>.b-cdn.net`.
- Valider phase 2 sur `https://media-dev.filfinance.fr`.
- Verifier les playlists replay, clips et calendar summary si des contenus sont
  disponibles.
- Lancer les paliers `50,100,200,500` pendant `60000ms`.
- Lancer le palier `1000` uniquement apres un palier `500` propre.
- Collecter erreurs HTTP, latences playlists, latences segments et bytes transferes.
- Archiver les commandes et sorties de validation.

## Hors scope

- Validation production.
- Test Cloudflare HLS.
- Changement de pricing ou contrat Bunny.
- Optimisation backend Rust.

## Approche technique

- Utiliser `FILFINANCE_LOAD_TEST_SKIP_WS=true` pour isoler le data plane HLS.
- Utiliser `FILFINANCE_LOAD_TEST_HLS_BASE_URL` pour pointer Bunny.
- Utiliser `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS=media-dev.filfinance.fr` uniquement
  pendant la phase 2 custom domain.
- Surveiller les codes HTTP non 2xx/3xx, les timeouts et les erreurs CORS cote client.
- Stopper la campagne si les erreurs HTTP deviennent significatives au palier courant.

## Criteres d'acceptation

- `https://api-dev.filfinance.fr/audio/replay/today/playlist.m3u8` ne retourne plus
  `hls replay origin is disabled`.
- Phase 1: smoke HTTP Bunny hostname OK.
- Phase 1: paliers `50,100,200,500` termines sans erreur HTTP significative.
- Phase 2: `media-dev.filfinance.fr` resout en DNS-only vers Bunny.
- Phase 2: certificat Bunny valide.
- Phase 2: memes smokes et paliers que phase 1 valides.
- Palier `1000` documente ou explicitement reporte avec raison.
- Les preuves montrent que `/hls/audio/...` n'est jamais appele.

## Tests attendus

Phase 1:

```bash
curl -I https://<pull-zone>.b-cdn.net/audio/replay/today/playlist.m3u8
FILFINANCE_LOAD_TEST_SKIP_WS=true \
FILFINANCE_LOAD_TEST_HLS_BASE_URL=https://<pull-zone>.b-cdn.net \
FILFINANCE_LOAD_TEST_TIERS=50,100,200,500 \
FILFINANCE_LOAD_TEST_DURATION_MS=60000 \
node scripts/load_backend_ws_hls.mjs
```

Phase 2:

```bash
curl -I https://media-dev.filfinance.fr/audio/replay/today/playlist.m3u8
FILFINANCE_LOAD_TEST_SKIP_WS=true \
FILFINANCE_LOAD_TEST_HLS_BASE_URL=https://media-dev.filfinance.fr \
FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS=media-dev.filfinance.fr \
FILFINANCE_LOAD_TEST_TIERS=50,100,200,500 \
FILFINANCE_LOAD_TEST_DURATION_MS=60000 \
node scripts/load_backend_ws_hls.mjs
```

## Dependances

- `EPIC-035`
- `NEWS-203`
- `NEWS-204`
- `NEWS-205`
