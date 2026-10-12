# NEWS-167 - Validation charge combinee Edge metadata et HLS

## Objectif

Valider le chemin production complet cote lecture:

- metadata live via Cloudflare Durable Objects;
- audio HLS replay ou clip via Worker HLS proxy;
- tickets HLS fournis par le backend Rust;
- backend Rust hors fan-out metadata.

## Scope

- Lancer en parallele:
  - `filfinance-edge/scripts/load-test-live.js`;
  - `scripts/load_backend_ws_hls.mjs`.
- Utiliser le meme palier pour metadata et HLS.
- Publier des messages metadata pendant toute la duree du run HLS.
- Reporter les resultats separement pour ne pas melanger les goulots.

## Hors scope

- Ingestion FinancialJuice.
- TTS et generation audio temps reel.
- Benchmark navigateur reel.
- Validation d'un domaine HLS dedie autre que le Worker courant.

## Commande cible

Note `EPIC-034`: le volet Edge metadata peut rester sur Cloudflare, mais le volet HLS
doit utiliser une cible media approuvee non Cloudflare.

Depuis la racine du repo:

```bash
FILFINANCE_EDGE_WS_URL="wss://filfinance-live-edge.sberoudblanc.workers.dev/ws/live" \
FILFINANCE_EDGE_PUBLISH_URL="https://filfinance-live-edge.sberoudblanc.workers.dev/publish/live" \
FILFINANCE_EDGE_PUBLISH_SECRET="<publish-secret>" \
FILFINANCE_LOAD_TEST_EMAIL="<listener-email>" \
FILFINANCE_LOAD_TEST_PASSWORD="<password>" \
FILFINANCE_LOAD_TEST_HLS_BASE_URL="https://media-dns-only.example.test" \
FILFINANCE_COMBINED_LOAD_TEST_TIERS=500 \
FILFINANCE_COMBINED_LOAD_TEST_DURATION_MS=60000 \
node scripts/load_edge_ws_hls_combined.mjs
```

Palier 1000 dans un second run apres validation du palier 500:

```bash
FILFINANCE_COMBINED_LOAD_TEST_TIERS=1000 \
node scripts/load_edge_ws_hls_combined.mjs
```

## Variables

- `FILFINANCE_COMBINED_LOAD_TEST_TIERS`: palier unique applique aux deux scripts, defaut `500`.
- `FILFINANCE_COMBINED_LOAD_TEST_DURATION_MS`: duree HLS et base de calcul des messages Edge, defaut `60000`.
- `FILFINANCE_EDGE_LOAD_TEST_MESSAGES`: surcharge explicite du nombre de messages Edge.
- `FILFINANCE_EDGE_LOAD_TEST_PUBLISH_INTERVAL_MS`: intervalle de publication Edge, defaut `500`.
- `FILFINANCE_LOAD_TEST_SKIP_WS`: force a `true` par defaut dans l'orchestrateur, car le WebSocket teste est celui du Worker Edge.
- `FILFINANCE_LOAD_TEST_HTTP_RETRY_ATTEMPTS`: retries HLS sur erreurs fetch reseau uniquement, defaut `2`.
- `FILFINANCE_LOAD_TEST_HTTP_RETRY_BACKOFF_MS`: backoff lineaire des retries HLS, defaut `250`.
- `FILFINANCE_EDGE_LOAD_TEST_TICKET`: ignore par defaut pour eviter les tickets expires dans les fichiers `.env`.
- `FILFINANCE_EDGE_LOAD_TEST_USE_MANUAL_TICKET`: `false` par defaut. Passer a `true` seulement pour forcer l'utilisation de `FILFINANCE_EDGE_LOAD_TEST_TICKET`; sinon l'orchestrateur recupere un ticket frais via `/api/edge/live-ticket`.
- `FILFINANCE_LOAD_TEST_ACCESS_TOKEN`: optionnel. Si les credentials email/password sont presents,
  l'orchestrateur les prefere pour obtenir un token frais au demarrage; cela evite les 401
  sur les refresh de tickets Edge pendant les runs longs.
- `FILFINANCE_EDGE_LOAD_TEST_TICKET_URL` et `FILFINANCE_EDGE_LOAD_TEST_ACCESS_TOKEN`: injectes automatiquement par l'orchestrateur dans le script Edge pour rafraichir le ticket pendant les probes longs.

## Criteres d'acceptation

- Rapport final `combined_load_test_done.report.ok=true`.
- Enfant `edge.code=0`:
  - connexions demandees toutes ouvertes;
  - `missed_deliveries=0`;
  - snapshot `hello` valide, avec `latest_event_id` attendu ou un `latest_event_id`
    externe non `load-test:*` si du trafic reel concurrent est observe;
  - heartbeat source valide.
- Enfant `hls.code=0`:
  - `ticket.failures=0`;
  - `playlist.failures=0`;
  - `segment.failures=0`;
  - `PLAYLIST_HIT` majoritaire;
  - `HIT` segment majoritaire.
- Logs backend Rust sans fan-out WebSocket metadata final.

## Implementation

`scripts/load_edge_ws_hls_combined.mjs` lance les deux scripts en parallele et encapsule
leurs sorties JSON avec:

```json
{ "stage": "combined_child_output", "child": "hls|edge", "payload": {} }
```

Le script ne masque pas les erreurs des enfants. Si un enfant sort en code non nul, le run
combine sort aussi en erreur.

Le probe snapshot Edge distingue maintenant:

- `latest_event_id_matches_expected`: dernier evenement strictement egal au dernier
  message publie par le test;
- `latest_event_id_compatible`: vrai si le dernier evenement est celui du test ou un
  evenement externe non `load-test:*`.

Cette distinction evite de classer en echec un run ou toutes les livraisons WebSocket
du test sont recues, mais ou un message reel concurrent devient le dernier evenement du
snapshot avant le probe.

## Resultats observes

### Palier 500 - valide

Run du 2026-05-11 12:36 UTC:

- Rapport combine: `ok=true`, `hls.code=0`, `edge.code=0`.
- Edge: `connected=500/500`, `missed_deliveries=0`, snapshot `ok=true`,
  heartbeat `ok=true`, latence p50 `122.07ms`, p95 `875.2ms`, p99 `1659.49ms`.
- HLS: playlist `5904` requetes, `0` failure, `PLAYLIST_HIT=5448`,
  `PLAYLIST_MISS=456`; segments `17712` requetes, `0` failure, `HIT=17612`.
- HLS retries reseau absorbes: playlist `44` tentatives, `33` succes apres retry;
  segments `74` tentatives, `50` succes apres retry.

### Palier 1000 - non valide en combine

Run du 2026-05-11 12:41 UTC:

- Rapport combine: `ok=false`, `hls.code=1`, `edge.code=1`.
- Edge transport: `connected=1000/1000`, `connection_failures=0`,
  `unexpected_closes=0`, `missed_deliveries=0`, `received_deliveries=120000/120000`.
- Edge latence sous forte charge: p50 `1637.64ms`, p95 `4181.18ms`,
  p99 `6583.34ms`, max `18325.27ms`.
- Edge snapshot avant correction du harness: `latest_event_id` observe
  `translated_news_item:00000000000000003068`, dernier evenement attendu
  `load-test:source_status:1778503636858:00120`; heartbeat `ok=true`.
- HLS: tickets `1000/1000` en HTTP 200, mais p95 ticket `23933.37ms`.
- HLS playlist: `10295` requetes, `10264` HTTP 200, `31` failures reseau finales
  apres `318` retries, dont `239` succes apres retry.
- HLS segments: `30792` requetes, `30700` HTTP 200, `92` failures reseau finales
  apres `1098` retries, dont `787` succes apres retry.

Interpretation: le palier 1000 ne montre pas de perte de fan-out Edge, mais depasse la
stabilite du harness HLS/client dans cette configuration. Le prochain run 1000 doit etre
refait avec le probe snapshot corrige et, si l'objectif est de valider le service plutot
que la machine de test, avec plus de retries HLS ou une cadence de polling HLS reduite.

### Palier 1000 / 120s - non valide

Run du 2026-05-11 13:02 UTC:

- Rapport combine: `ok=false`, `hls.code=1`, `edge.code=1`.
- Edge: `connected=966/1000`, `connection_failures=34`; le script a ensuite echoue
  sur un refresh de ticket Edge en HTTP 401 pendant le probe snapshot.
- HLS: tickets `1000/1000` en HTTP 200, mais p95 ticket `23123.18ms`.
- HLS playlist: `15713` requetes, `15547` HTTP 200, `166` failures finales apres
  `2211` retries.
- HLS segments: `46641` requetes, `46429` HTTP 200, `212` failures finales apres
  `2857` retries.

Correctif harness associe: le script Edge peut renouveler son token Supabase via
email/password quand le refresh de ticket Edge renvoie 401/403, et les probes finaux
reportent l'erreur au lieu de faire perdre le rapport de palier.

## Dependances

- `NEWS-161`
- `NEWS-162`
- `NEWS-165`
