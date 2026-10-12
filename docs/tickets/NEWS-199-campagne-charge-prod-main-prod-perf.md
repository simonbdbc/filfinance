# NEWS-199 - Campagne charge prod main vs prod-perf

## Objectif

Mesurer en production reelle la capacite actuelle, puis comparer le gain apres deploiement
de la branche `prod-perf` sur le nouveau serveur.

La campagne est lancee depuis le serveur cible et mesure les URLs publiques API/Edge/HLS, mais
HLS ne doit plus passer par le proxy Cloudflare `live.filfinance.fr`. La cible media doit etre
fournie explicitement et autorisee par `FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS`.

## Branches

- Baseline: `main`, serveur actuel, sans optimisation prealable.
- Optimisation: `prod-perf`, branche creee depuis `main`.
- Toutes les ameliorations code et configuration serveur sont faites sur `prod-perf`.
- La prod amelioree doit utiliser le nouveau serveur: GPU NVIDIA, double processeur multi-core,
  128 Go de memoire.

## Harness

Script ajoute:

```bash
node scripts/load_prod_full_pipeline.mjs --dry-run
```

Run reel:

```bash
FILFINANCE_PROD_LOAD_CONFIRM=prod-real \
FILFINANCE_PROD_LOAD_PROFILE=main \
FILFINANCE_EDGE_PUBLISH_SECRET="<edge-publish-secret>" \
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://<media-host>" \
FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS="<media-host>" \
FILFINANCE_LOAD_TEST_ACCESS_TOKEN="<moderator-or-admin-token>" \
node scripts/load_prod_full_pipeline.mjs
```

Pour la campagne amelioree:

```bash
FILFINANCE_PROD_LOAD_CONFIRM=prod-real \
FILFINANCE_PROD_LOAD_PROFILE=prod-perf \
FILFINANCE_EDGE_PUBLISH_SECRET="<edge-publish-secret>" \
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://<media-host>" \
FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS="<media-host>" \
FILFINANCE_LOAD_TEST_ACCESS_TOKEN="<moderator-or-admin-token>" \
node scripts/load_prod_full_pipeline.mjs
```

Le script refuse tout run tant que `FILFINANCE_LOAD_TEST_HLS_BASE_URL` est absent, et refuse
Cloudflare HLS sauf override explicite `FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST=true`.
Un run reel exige en plus `FILFINANCE_PROD_LOAD_CONFIRM=prod-real`.

## Surface testee

- Edge WebSocket live: `wss://live.filfinance.fr/ws/live`.
- HLS public: URL media explicite, typiquement Bunny/CDN media, jamais `https://live.filfinance.fr`
  par defaut.
- Backend API: `https://api.filfinance.fr`.
- WebSocket backend test-runs: `wss://api.filfinance.fr/ws/test-runs`.
- Injections synthetiques via `/ws/test-runs` toutes les `15s`.
- Probes runtime:
  - `/health`
  - `/api/admin/health`
  - `/api/runtime/translation`
  - `/api/runtime/tts`
  - `/api/runtime/hls`
- Workflow calendrier:
  - `POST /api/admin/calendar-day/run`
  - `POST /api/admin/calendar-week/run`

## Paliers et seuils

Paliers par defaut:

```text
50 warmup, 100 baseline, 200, 500, 750, 1000, 1500, 2000
```

Sur `prod-perf`, le harness ajoute par defaut:

```text
2500, 3000
```

La metrique principale est:

```text
hls_audio_delivery_p95_ms = max(ticket.p95, playlist.p95, segment.p95)
```

Le taux d'erreur HLS est:

```text
hls_error_rate =
  (ticket.failures + playlist.failures + segment.failures)
  / (ticket.requests + playlist.requests + segment.requests)
```

Arret:

- stop immediat si `hls_error_rate >= 10%`;
- stop si `hls_audio_delivery_p95_ms >= baseline_p95_ms * 1.20`;
- le seuil latence est confirme par un second run du meme palier avant arret.

## Donnees collectees

Le harness ecrit par defaut dans:

```text
/tmp/filfinance-load-tests/<run-id>/
```

Fichiers:

- `events.jsonl`: evenements complets, sorties enfants, probes runtime, snapshots systeme.
- `summary.json`: synthese par palier.

Chaque palier contient:

- clients demandes;
- latences HLS ticket / playlist / segment;
- taux d'erreur HLS;
- latence Edge p95;
- livraisons Edge manquees;
- test-runs envoyes / acceptes;
- probes traduction / TTS / HLS;
- charge locale du generateur;
- snapshot systeme, Docker et NVIDIA si disponible.

Un palier est marque `generator_limited` si le generateur local consomme trop le serveur
pour que le resultat soit interpretable comme limite applicative.

## Calendrier n8n

- `calendar-day` est lance aux paliers `100` et `500`.
- `calendar-week` est lance une fois au palier `500`.
- Un `calendar-day` supplementaire est lance apres campagne sur le dernier palier stable si ce palier
  n'etait pas deja couvert.
- Les raisons sont taguees:

```text
load_test_<profile>_<run-id>_<kind>_<tier>
```

## Optimisations prevues sur prod-perf

- App globale:
  - supprimer fetchs redondants;
  - verifier absence de polling HLS idle;
  - auditer requetes lentes et pools API/DB;
  - verifier cache frontend.
- Audio/HLS:
  - ajuster cache Worker/CDN playlist et segments;
  - verifier retry/backoff;
  - conserver `#EXT-X-ENDLIST` sur les clips finalises;
  - suivre separement ticket, playlist, segment.
- Traduction/TTS:
  - benchmarker `quality` vs `balanced`;
  - dimensionner `NEWS_TRANSLATION_WORKERS` et `NEWS_TTS_WORKERS` selon les coeurs reels;
  - tester CUDA seulement si le runtime installe le supporte sans regression qualite;
  - sinon exploiter CPU multi-core.
- n8n:
  - conserver l'orchestration Rust sequentielle;
  - reduire les payloads;
  - ajouter timings par node;
  - ajuster la memoire Node/n8n si OOM confirme.

## Comparaison attendue

Le rapport final doit comparer `current dev` et `improved prod-perf` sur:

- dernier palier stable;
- premier palier a `>=10%` erreurs HLS;
- premier palier a `+20%` p95 HLS;
- p95 HLS aux paliers fixes `500`, `1000`, `1500`, `2000`;
- latence news synthetique -> traduction -> TTS -> HLS -> audio_ready;
- duree et stabilite n8n jour/semaine;
- CPU/RAM/disque/reseau;
- cout du generateur local;
- gain absolu et gain normalise par vCPU/RAM.

## Verification

Avant tout run reel:

```bash
node --check scripts/load_prod_full_pipeline.mjs
node --check scripts/report_perf_comparison.mjs
node scripts/load_prod_full_pipeline.mjs --dry-run
```
