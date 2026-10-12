# NEWS-201 - Garde scripts charge HLS Cloudflare

## Objectif

Empecher les scripts de charge HLS de cibler accidentellement Cloudflare ou la production
`filfinance.fr`.

## Scope

- Bloquer les bases HLS contenant:
  - `filfinance.fr`;
  - `workers.dev`;
  - `cloudflarestream.com`;
  - `videodelivery.net`.
- Appliquer le blocage avant toute requete reseau.
- Autoriser un contournement explicite par
  `FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST=true`.
- Laisser les URLs locales et DNS-only non Cloudflare fonctionner.

## Criteres d'acceptation

- `FILFINANCE_LOAD_TEST_HLS_BASE_URL=https://live.filfinance.fr` sort en erreur avant
  auth ou fetch.
- Une base locale reste autorisee.
- L'opt-in explicite contourne la garde.
- `scripts/load_edge_ws_hls_combined.mjs` herite de la protection via le script HLS.

## Dependances

- `EPIC-034`
