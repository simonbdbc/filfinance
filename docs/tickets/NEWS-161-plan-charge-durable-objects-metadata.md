# NEWS-161 - Plan charge Durable Objects metadata

## Objectif

Valider que le control plane live supporte la charge utilisateur via Cloudflare Durable Objects, sans faire porter le fan-out au backend Rust.

## Scope

- Tester `/ws/live` sur Cloudflare Worker.
- Publier des messages representatifs via `/publish/live`.
- Simuler plusieurs paliers:
  - 100 clients;
  - 500 clients;
  - 1000 clients;
  - 2000 clients si le palier 1000 est stable.
- Mesurer les livraisons et latences.
- Verifier le snapshot de reconnexion.
- Verifier que Rust ne maintient aucune socket metadata client finale pendant le test.

## Hors scope

- Charge HLS audio.
- Charge ingestion FinancialJuice.
- Tests de traduction/TTS.

Le scenario combine metadata Edge + HLS Worker est traite separement par `NEWS-167`.

## Messages a tester

- `translated_news_item`;
- `audio_ready`;
- `audio_failed`;
- `audio_finalized`;
- `system_notice`;
- `source_status` ou `sources` via `hello`/`heartbeat`.

## Mesures

- clients demandes;
- clients connectes;
- echecs de connexion;
- messages publies;
- livraisons attendues;
- livraisons recues;
- livraisons manquantes;
- latence p50/p95/p99/max;
- reconnexions;
- taille du snapshot.

## Criteres d'acceptation

- 1000 clients recoivent les messages sans perte observable.
- p95 et p99 sont archives.
- Le backend Rust publie une seule copie de chaque evenement vers le Worker.
- Le backend Rust ne maintient pas les 1000 sockets metadata.
- Les badges sources publics se mettent a jour via le control plane.

## Commande cible

```bash
cd filfinance-edge
FILFINANCE_EDGE_WS_URL="wss://live.filfinance.fr/ws/live" \
FILFINANCE_EDGE_LOAD_TEST_TICKET="<edge-live-ticket>" \
FILFINANCE_EDGE_PUBLISH_URL="https://live.filfinance.fr/publish/live" \
FILFINANCE_EDGE_PUBLISH_SECRET="<publish-secret>" \
FILFINANCE_EDGE_LOAD_TEST_TIERS=100,500,1000 \
npm run load-test:live
```

`FILFINANCE_EDGE_LOAD_TEST_CLIENTS=1000` reste supporte pour lancer un seul palier.

## Implementation

Le script `filfinance-edge/scripts/load-test-live.js` execute maintenant un ou plusieurs paliers
via `FILFINANCE_EDGE_LOAD_TEST_TIERS`.

Pour chaque palier, il:

- ouvre les clients `/ws/live?ticket=...`;
- publie des messages representatifs:
  - `translated_news_item`;
  - `audio_ready`;
  - `audio_failed`;
  - `audio_finalized`;
  - `system_notice`;
  - `source_status`;
- mesure les livraisons attendues/recues/manquantes;
- mesure les latences p50/p95/p99/max;
- verifie le snapshot recu dans `hello`;
- verifie que le `heartbeat` expose les sources quand un `source_status` a ete publie.

Validation locale:

```bash
cd filfinance-edge
npm run check
```

## Dependances

- `EPIC-026`
- `EPIC-018`
- `NEWS-157`
- `NEWS-158`
- `NEWS-167`
