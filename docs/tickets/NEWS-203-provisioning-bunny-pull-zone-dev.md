# NEWS-203 - Provisioning Bunny Pull Zone dev

## Objectif

Creer la Bunny Pull Zone dev `filfinance-dev-hls` pour servir le data plane audio HLS de
l'environnement `dev`, sans passer par le CDN Cloudflare standard.

## Scope

- Creer la Pull Zone `filfinance-dev-hls`.
- Configurer l'origin `https://api-dev.filfinance.fr`.
- Verifier les chemins HLS backend:
  - `/audio/replay/today/...`;
  - `/audio/clips/...`;
  - `/audio/calendar-summary/...`.
- Configurer Bunny pour ne pas ignorer les query strings.
- Ne pas override globalement `Cache-Control`.
- Activer CORS pour `.m3u8` et `.ts`.
- Bloquer POST.
- Definir une limite mensuelle dev.
- Documenter le runbook phase 1 `https://<pull-zone>.b-cdn.net`.
- Documenter le runbook phase 2 `media-dev.filfinance.fr` avec CNAME Cloudflare DNS-only
  et certificat Bunny.

## Hors scope

- Bunny Storage.
- Migration production.
- Reactivation du Worker HLS `/hls/audio/...`.
- Changement de l'origin Rust.

## Approche technique

- Utiliser une Pull Zone Bunny standard devant `https://api-dev.filfinance.fr`.
- Garder le backend Rust comme producteur et origin des manifests et segments.
- Conserver les headers HLS origin existants et eviter les overrides CDN globaux.
- Ajouter `media-dev.filfinance.fr` comme custom hostname uniquement apres validation du
  hostname Bunny.
- Garder le CNAME Cloudflare en DNS-only pour que le trafic media ne repasse pas par
  Cloudflare.

## Criteres d'acceptation

- `https://<pull-zone>.b-cdn.net/audio/replay/today/playlist.m3u8` repond via Bunny.
- Si la reponse est `404` avec `hls replay origin is disabled`, le blocage CDN est leve
  mais `.env.server.dev` doit encore activer `NEWS_TTS_ENABLED=true` et
  `NEWS_HLS_ENABLED=true`.
- Les URLs de playlists et segments restent en `/audio/...`.
- Aucune URL ne contient `/hls/audio/...`.
- `media-dev.filfinance.fr` est documente comme DNS-only.
- Le certificat Bunny est charge et verifie avant bascule phase 2.
- Une limite mensuelle dev est configuree.

## Tests attendus

```bash
curl -I https://<pull-zone>.b-cdn.net/audio/replay/today/playlist.m3u8
curl -fsS https://<pull-zone>.b-cdn.net/audio/replay/today/playlist.m3u8 | grep -q "#EXTM3U"
```

Apres phase 2:

```bash
curl -I https://media-dev.filfinance.fr/audio/replay/today/playlist.m3u8
curl -fsS https://media-dev.filfinance.fr/audio/replay/today/playlist.m3u8 | grep -q "#EXTM3U"
```

## Dependances

- `EPIC-035`
- `EPIC-034`
