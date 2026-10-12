# NEWS-205 - Garde scripts charge compatible Bunny

## Objectif

Adapter la garde des scripts de charge HLS pour autoriser Bunny dev/prod sans affaiblir
le blocage Cloudflare.

## Scope

- Modifier `scripts/load_backend_ws_hls.mjs`.
- Supprimer le prefixage automatique `/hls` devant `/audio/...` dans
  `resolveHlsTargetUrl`.
- Garder le blocage des cibles Cloudflare:
  - `live*.filfinance.fr`;
  - `workers.dev`;
  - `cloudflarestream.com`;
  - `videodelivery.net`.
- Autoriser `*.b-cdn.net` par defaut.
- Ajouter `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS` comme liste CSV de hostnames media
  explicitement autorises.
- Autoriser `media-dev.filfinance.fr` et `media.filfinance.fr` uniquement via
  `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS`.
- Garder `FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST` reserve aux tests Cloudflare
  valides.

## Hors scope

- Campagne de charge effective.
- Provisioning Bunny.
- Modifications du backend Rust.

## Approche technique

- Parser la base HLS avec `URL` et comparer le hostname normalise.
- Autoriser les hostnames finissant par `.b-cdn.net` ou egaux a `b-cdn.net`.
- Bloquer `filfinance.fr` par defaut, sauf hostname present dans
  `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS`.
- Appliquer la garde avant toute auth ou requete reseau.
- Conserver l'opt-in Cloudflare existant pour les cas de validation ecrite Cloudflare
  seulement.

## Criteres d'acceptation

- `FILFINANCE_LOAD_TEST_HLS_BASE_URL=https://example.b-cdn.net` est autorise.
- `FILFINANCE_LOAD_TEST_HLS_BASE_URL=https://live-dev.filfinance.fr` est bloque avant
  toute requete reseau.
- `FILFINANCE_LOAD_TEST_HLS_BASE_URL=https://media-dev.filfinance.fr` est bloque sans
  `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS`.
- `FILFINANCE_LOAD_TEST_HLS_BASE_URL=https://media.filfinance.fr` est bloque sans
  `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS`.
- `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS=media-dev.filfinance.fr` autorise
  `https://media-dev.filfinance.fr`.
- `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS=media.filfinance.fr` autorise
  `https://media.filfinance.fr`.
- Les URLs construites par le script restent en `/audio/...`, jamais `/hls/audio/...`.
- `FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST=true` ne devient pas la voie nominale
  Bunny.

## Tests attendus

```bash
node --check scripts/load_backend_ws_hls.mjs
```

Tests statiques ou commandes controlees avec `FILFINANCE_LOAD_TEST_SKIP_WS=true` et
`FILFINANCE_LOAD_TEST_SKIP_HLS=true` pour verifier les decisions de garde sans lancer de
charge reseau.

## Dependances

- `EPIC-035`
- `NEWS-204`
