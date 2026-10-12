# NEWS-204 - Resolver client HLS generique et CI dev

## Objectif

Rendre le resolver HLS client compatible avec une base media generique Bunny, et corriger
la CI dev pour ne plus fallback vers `live-dev.filfinance.fr`.

## Scope

- Modifier `filfinance-client/src/shared/api/urls.ts`.
- Supprimer le prefixage automatique `/hls` devant les chemins `/audio/...`.
- Conserver le comportement base HLS vide vers l'API backend derivee de
  `EXPO_PUBLIC_FILFINANCE_WS_URL`.
- Ajouter ou adapter les tests autour de `resolveHlsResourceUrl`.
- Mettre a jour `client:web-build` dans `.gitlab-ci.yml`.
- Sur `dev`, utiliser `FILFINANCE_DEV_HLS_BASE_URL` si present, sinon conserver
  `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL` si elle est deja fournie.
- Sur `dev`, ne pas fallback vers `https://live-dev.filfinance.fr`.
- Sur `main`, garder `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL` vide.

## Hors scope

- Provisioning Bunny.
- Campagne de charge.
- Migration production.
- Refonte du player audio.

## Approche technique

- Traiter `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL` comme une base URL brute.
- Resoudre les ressources HLS avec leur chemin canonique `/audio/...`.
- Ne conserver `/hls/audio/...` que comme reference historique Cloudflare dans les docs.
- Eviter tout fallback CI vers `live*.filfinance.fr` pour le data plane HLS.

## Criteres d'acceptation

- Base HLS vide: `buildReplayTodayPlaylistUrl` retourne une URL backend API en
  `/audio/replay/today/playlist.m3u8`.
- Base HLS `https://example.b-cdn.net`: `buildReplayTodayPlaylistUrl` retourne
  `https://example.b-cdn.net/audio/replay/today/playlist.m3u8`.
- Une ressource `/audio/clips/...` reste `/audio/clips/...`.
- Aucune base Bunny ne produit `/hls/audio/...`.
- `client:web-build` sur `dev` n'injecte pas `https://live-dev.filfinance.fr` comme base
  HLS.
- `client:web-build` sur `main` n'injecte pas `https://live.filfinance.fr` comme base HLS.

## Tests attendus

- Tests unitaires ou statiques du resolver HLS client.
- `npm run typecheck` dans `filfinance-client`.
- Verification de `.gitlab-ci.yml` montrant:
  - `dev` utilise `FILFINANCE_DEV_HLS_BASE_URL` ou la variable publique deja fournie;
  - `main` garde `EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL` vide;
  - aucune base HLS CI ne pointe vers `live*.filfinance.fr`.

## Dependances

- `EPIC-035`
- `NEWS-203`
